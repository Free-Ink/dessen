// Reading stats: widget cards built from the firmware's reading session log
// (/.crosspoint/reading_sessions.jsonl, one JSON object per line — see
// crosspoint-reader src/sync/ReadingSessionLog.h). The log is never trimmed, so
// it is streamed through a small buffer with dsn_fs_read_at, not loaded whole.
//
// It is also a home launcher ("home": true in the manifest): the book being
// read is a card pinned to the bottom of page 1, and every later page ends in a grid of
// app icons for the firmware's own screens.
//
// Widgets flow top to bottom and spill onto further pages, so the same code
// fits any panel. Swipes (or a tap on empty space) page. NEXT/PREV step a
// focus through the book card, then each icon on page 2, then the remaining
// pages (their icons are the same ones, there for touch);
// CONFIRM or a tap opens what is focused or touched. BACK returns to page 1.
// Build + check: make rust
#![no_std]
#![allow(dead_code)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    dsn_exit(99);
    loop {}
}

include!("../sdk/dessen.rs");
include!("readstats_icons.rs"); // TILES, TILE (tools/readstats_icons.py)

// sdk/dessen_abi.h
const BLACK: i32 = 1;
const LIGHT: i32 = 2;
const DARK: i32 = 3;
const SMALL: i32 = 0;
const BODY: i32 = 1;
const TITLE: i32 = 2;
const LEFT: i32 = 0;
const CENTER: i32 = 1;
const RIGHT: i32 = 2;
const BOLD: i32 = 1;
const ERR_UNAVAILABLE: i32 = -4;

const LOG: &[u8] = b"/.crosspoint/reading_sessions.jsonl";
const DAYS: usize = 53 * 7; // heatmap history, newest last
const NBOOKS: usize = 64;
const G: i32 = 28; // side gutter
const PAD: i32 = 12; // card padding
const GAP: i32 = 12; // between cards
const WIDGETS: usize = 6;
const STATUS_Y: i32 = 10; // top of the clock / battery line
const COVER_MAX: i32 = 240;
// Launcher grid, in TILES order: label, open_home destination (DSN_HOME_*).
const APPS: [(&[u8], i32); 6] =
    [(b"Library", 1), (b"Files", 0), (b"OPDS", 3), (b"Plugins", 2), (b"Transfer", 4), (b"Settings", 5)];
const NAPPS: i32 = APPS.len() as i32;

// The book to continue: the first row of the recents shelf.
struct Home {
    rec: [u8; 1100],        // "path\ttitle\tauthor\t..." (DSN_SHELF_RECORD_MAX)
    f: [(usize, usize); 3], // path, title, author as ranges of rec
    percent: i32,           // 0..=100, or -1 if unknown
    cover: [u8; 8192],      // {u16 w, u16 h} + 1-bit rows, from dsn_book_cover
    cover_n: usize,
    card: (i32, i32),       // y, height of the card on page 1
    ring: bool,             // draw the focus ring (boards with buttons)
}

impl Home {
    fn field(&self, i: usize) -> &[u8] {
        &self.rec[self.f[i].0..self.f[i].1]
    }
    fn has_book(&self) -> bool {
        self.field(0).first() == Some(&b'/')
    }
    fn load(&mut self) {
        let n = dsn_recent_shelf(0, 1, &mut self.rec);
        let n = if n > 0 { (n as usize).min(self.rec.len()) } else { 0 };
        let (mut start, mut k) = (0, 0);
        for i in 0..n {
            let c = self.rec[i];
            if c == b'\t' || c == b'\n' {
                if k < 3 {
                    self.f[k] = (start, i);
                    k += 1;
                }
                start = i + 1;
                if c == b'\n' {
                    break;
                }
            }
        }
        if k < 2 {
            self.f[0] = (0, 0); // not a whole record
        }
        if self.has_book() {
            let p = dsn_book_progress(self.field(0));
            self.percent = if (0..=100).contains(&p) { p } else { -1 };
        }
    }
}

#[derive(Clone, Copy)]
struct Book {
    key: u32,
    secs: u32,
    title: [u8; 80],
    tlen: usize,
}

struct Stats {
    end: u32,         // local day number (days since 1970) of day[DAYS - 1]: today
    day: [u32; DAYS], // seconds read per local day
    hour: [u32; 24],  // seconds by local hour the session started in
    books: [Book; NBOOKS],
    nbooks: usize,
    total: u32,
    sessions: u32,
    progress_bp: u32, // forward book progress, summed, in basis points
    err: i32,
}

// The value after `"key":`. A plain substring search is sound here: a quote
// inside a title or path is escaped (\"), so the pattern can't match in a string.
fn field<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    line.windows(key.len()).position(|w| w == key).map(|p| &line[p + key.len()..])
}

fn num(line: &[u8], key: &[u8]) -> i64 {
    let Some(v) = field(line, key) else { return 0 };
    let neg = v.first() == Some(&b'-');
    let mut n: i64 = 0;
    for &c in v[neg as usize..].iter().take(12) {
        if !c.is_ascii_digit() {
            break;
        }
        n = n * 10 + (c - b'0') as i64;
    }
    if neg { -n } else { n }
}

impl Stats {
    fn shift(&mut self, n: u32) {
        let n = (n as usize).min(DAYS);
        self.day.copy_within(n.., 0);
        self.day[DAYS - n..].fill(0);
    }

    fn add(&mut self, line: &[u8], have_clock: bool) {
        let start = num(line, b"\"start\":");
        let secs = num(line, b"\"secs\":");
        // Same sanity limits the firmware applies before uploading a session.
        if start < 946_684_800 || start > 0xf000_0000 || secs <= 0 || secs > 7 * 86400 {
            return;
        }
        let secs = secs as u32;
        let local = (start + num(line, b"\"tz\":").clamp(-720, 840) * 60) as u32;
        let d = local / 86400;
        if d > self.end {
            if have_clock {
                return; // dated in the future: measured with a wrong clock
            }
            // No clock: "today" is the newest session seen so far.
            self.shift(d - self.end);
            self.end = d;
        }
        // ponytail: a session counts wholly toward the day and hour it started in;
        // split at midnight/hour boundaries if late-night sessions look wrong.
        let ago = (self.end - d) as usize;
        if ago < DAYS {
            self.day[DAYS - 1 - ago] += secs;
        }
        self.hour[(local % 86400 / 3600) as usize] += secs;
        self.total += secs;
        self.sessions += 1;
        let from = num(line, b"\"from\":").clamp(0, 10000);
        let to = num(line, b"\"to\":").clamp(0, 10000);
        if to > from {
            self.progress_bp += (to - from) as u32;
        }

        let Some(doc) = field(line, b"\"doc\":\"") else { return };
        let mut key: u32 = 0x811c_9dc5; // FNV-1a of the document id
        for &c in doc.iter().take_while(|&&c| c != b'"') {
            key = (key ^ c as u32).wrapping_mul(0x0100_0193);
        }
        let i = match self.books[..self.nbooks].iter().position(|b| b.key == key) {
            Some(i) => i,
            None => {
                // ponytail: past NBOOKS distinct books the least-read one is dropped,
                // so its earlier time is lost from "Most read" (totals are unaffected).
                let i = if self.nbooks < NBOOKS {
                    self.nbooks += 1;
                    self.nbooks - 1
                } else {
                    let mut min = 0;
                    for j in 1..NBOOKS {
                        if self.books[j].secs < self.books[min].secs {
                            min = j;
                        }
                    }
                    min
                };
                let mut b = Book { key, secs: 0, title: [0; 80], tlen: 0 };
                let mut t = field(line, b"\"title\":\"").unwrap_or(b"").iter();
                while let Some(&c) = t.next() {
                    // JSON string: stop at the closing quote, take an escaped byte literally.
                    let c = match c {
                        b'"' => break,
                        b'\\' => *t.next().unwrap_or(&b'?'),
                        c => c,
                    };
                    if b.tlen == b.title.len() {
                        break;
                    }
                    b.title[b.tlen] = c;
                    b.tlen += 1;
                }
                self.books[i] = b;
                i
            }
        };
        self.books[i].secs += secs;
    }

    fn load(&mut self, have_clock: bool) {
        let mut buf = [0u8; 2048];
        let (mut len, mut off, mut skip) = (0usize, 0u32, false);
        loop {
            let n = dsn_fs_read_at(LOG, off, &mut buf[len..]);
            if n <= 0 {
                // A missing log (IO error) just means nothing has been read yet.
                if n == ERR_UNAVAILABLE {
                    self.err = n;
                }
                return; // an unfinished last line is left out, as the firmware does
            }
            let n = (n as usize).min(buf.len() - len);
            off += n as u32;
            len += n;
            let mut s = 0;
            while let Some(p) = buf[s..len].iter().position(|&c| c == b'\n') {
                if !skip {
                    self.add(&buf[s..s + p], have_clock);
                }
                skip = false;
                s += p + 1;
            }
            buf.copy_within(s..len, 0);
            len -= s;
            if len == buf.len() {
                // A line longer than the buffer: drop it through its newline.
                len = 0;
                skip = true;
            }
        }
    }

    // (current, best) runs of consecutive days with reading. Today not having
    // been read yet doesn't break the current streak.
    fn streaks(&self) -> (u32, u32) {
        let (mut best, mut run) = (0, 0);
        for &d in self.day.iter() {
            run = if d > 0 { run + 1 } else { 0 };
            best = best.max(run);
        }
        let mut cur = 0;
        let skip = (self.day[DAYS - 1] == 0) as usize;
        for &d in self.day.iter().rev().skip(skip) {
            if d == 0 {
                break;
            }
            cur += 1;
        }
        (cur, best) // ponytail: "best" only sees the last 53 weeks
    }
}

// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm), y >= 1970.
fn days_from_civil(y: u32, m: u32, d: u32) -> u32 {
    let y = if m <= 2 { y - 1 } else { y };
    let (era, yoe) = (y / 400, y % 400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

// A tiny string builder for labels.
struct S {
    b: [u8; 24],
    n: usize,
}

impl S {
    fn new() -> S {
        S { b: [0; 24], n: 0 }
    }
    fn s(mut self, t: &[u8]) -> S {
        for &c in t {
            if self.n < self.b.len() {
                self.b[self.n] = c;
                self.n += 1;
            }
        }
        self
    }
    fn n(self, v: u32) -> S {
        let mut d = [0u8; 10];
        let (mut i, mut v) = (d.len(), v);
        loop {
            i -= 1;
            d[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.s(&d[i..])
    }
    fn get(&self) -> &[u8] {
        &self.b[..self.n]
    }
}

// "45m", "5h 45m", "123h"
fn dur(secs: u32) -> S {
    let (h, m) = (secs / 3600, secs / 60 % 60);
    match h {
        0 => S::new().n(m).s(b"m"),
        1..=99 => S::new().n(h).s(b"h ").n(m).s(b"m"),
        _ => S::new().n(h).s(b"h"),
    }
}

fn days(n: u32) -> S {
    S::new().n(n).s(if n == 1 { b" day" } else { b" days" })
}

fn text(x: i32, y: i32, w: i32, h: i32, s: &[u8], font: i32, align: i32, flags: i32) {
    dsn_text_box(x, y, w, h, s, font, align, 1, flags);
}

fn text_w(s: &[u8], font: i32, flags: i32) -> i32 {
    (dsn_measure_text(s, font, flags) >> 16) as i32
}

struct Ui {
    w: i32,
    h: i32,
    ls: i32, // line heights: small, body, title. Everything is sized from these,
    lb: i32, // so the layout scales with the board's fonts.
    lt: i32,
}

impl Ui {
    // Height of a card's label row; content starts this far below the card top.
    fn head(&self) -> i32 {
        PAD / 2 + self.ls + 6
    }

    fn card(&self, x: i32, y: i32, w: i32, h: i32, label: &[u8]) {
        dsn_stroke(x, y, w, h, BLACK, 1, 8, 15);
        text(x + PAD, y + PAD / 2, w - 2 * PAD, self.ls, label, SMALL, LEFT, 0);
    }

    // Four label/value tiles: one row on a wide screen, 2x2 otherwise.
    fn tiles(&self, y: i32, draw: bool, t: [(&[u8], S); 4]) -> i32 {
        let w = self.w - 2 * G;
        let cols = if w >= 640 { 4 } else { 2 };
        let th = self.head() + self.lt + PAD;
        let rows = 4 / cols;
        if draw {
            let tw = (w - (cols - 1) * GAP) / cols;
            for (i, (label, value)) in t.iter().enumerate() {
                let (c, r) = (i as i32 % cols, i as i32 / cols);
                let (x, y) = (G + c * (tw + GAP), y + r * (th + GAP));
                self.card(x, y, tw, th, label);
                let v = value.get();
                let font = if text_w(v, TITLE, BOLD) > tw - 2 * PAD { BODY } else { TITLE };
                text(x + PAD, y + self.head(), tw - 2 * PAD, self.lt, v, font, LEFT, BOLD);
            }
        }
        rows * th + (rows - 1) * GAP
    }

    // Draws widget `i` at `y` when `draw`; returns its height (0 = nothing to show).
    fn widget(&self, st: &Stats, i: usize, y: i32, draw: bool) -> i32 {
        let (x, w) = (G, self.w - 2 * G);
        let inner = w - 2 * PAD;
        let (ls, lb) = (self.ls, self.lb);
        let top = y + self.head();
        let week = &st.day[DAYS - 7..];
        let wd = (st.end + 4) % 7; // weekday of today, 0 = Sunday
        match i {
            0 => {
                let (cur, _) = st.streaks();
                let t: [(&[u8], S); 4] = [
                    (b"Today", dur(st.day[DAYS - 1])),
                    (b"Last 7 days", dur(week.iter().sum())),
                    (b"Streak", days(cur)),
                    (b"Total time", dur(st.total)),
                ];
                self.tiles(y, draw, t)
            }
            1 => {
                // Contribution grid: a column per week (newest right), Sunday on top.
                let cell = ls * 3 / 4;
                let pitch = cell + (cell / 4).max(2);
                let cols = ((inner + pitch - cell) / pitch).min(53);
                let h = self.head() + 7 * pitch - (pitch - cell) + PAD;
                if !draw {
                    return h;
                }
                self.card(x, y, w, h, S::new().s(b"Last ").n(cols as u32).s(b" weeks").get());
                let shade = |cx: i32, cy: i32, level: i32| match level {
                    0 => dsn_stroke(cx, cy, cell, cell, LIGHT, 1, 0, 0),
                    l => dsn_fill(cx, cy, cell, cell, [LIGHT, DARK, BLACK][l as usize - 1]),
                };
                for l in 0..4 {
                    shade(x + w - PAD - (4 - l) * pitch + (pitch - cell), y + PAD / 2 + (ls - cell) / 2, l);
                }
                let left = x + (w - cols * pitch + (pitch - cell)) / 2;
                for c in 0..cols {
                    for r in 0..7 {
                        let ago = (cols - 1 - c) * 7 + wd as i32 - r;
                        if ago < 0 {
                            continue; // later this week
                        }
                        let level = match st.day[DAYS - 1 - ago as usize] / 60 {
                            0 => 0,
                            1..=14 => 1,
                            15..=44 => 2,
                            _ => 3,
                        };
                        shade(left + c * pitch, top + r * pitch, level);
                    }
                }
                h
            }
            2 => {
                // Bars for the last seven days, minutes on top, weekday below.
                let ah = 4 * ls;
                let h = self.head() + ls + ah + 4 + ls + PAD;
                if !draw {
                    return h;
                }
                self.card(x, y, w, h, b"Last 7 days (minutes)");
                let max = week.iter().copied().max().unwrap_or(0).max(1);
                let cw = inner / 7;
                let base = top + ls + ah;
                for (k, &secs) in week.iter().enumerate() {
                    let cx = x + PAD + k as i32 * cw;
                    let bh = if secs > 0 { ((secs / 60 * ah as u32 / (max / 60).max(1)) as i32).clamp(2, ah) } else { 0 };
                    dsn_fill(cx + cw / 5, base - bh, cw * 3 / 5, bh, BLACK);
                    if secs > 0 {
                        text(cx, base - bh - ls, cw, ls, S::new().n(secs / 60).get(), SMALL, CENTER, 0);
                    }
                    let name = (wd as usize + 1 + k) % 7 * 2;
                    text(cx, base + 4, cw, ls, &b"SuMoTuWeThFrSa"[name..name + 2], SMALL, CENTER, 0);
                }
                dsn_line(x + PAD, base, x + w - PAD, base, 1, DARK);
                h
            }
            3 => {
                // When in the day reading happens: 24 bars, one per hour.
                let ah = 3 * ls;
                let h = self.head() + ah + 4 + ls + PAD;
                if !draw {
                    return h;
                }
                self.card(x, y, w, h, b"Time of day");
                let max = st.hour.iter().copied().max().unwrap_or(0).max(1);
                let cw = inner / 24;
                let left = x + (w - 24 * cw) / 2;
                let base = top + ah;
                for (k, &secs) in st.hour.iter().enumerate() {
                    let cx = left + k as i32 * cw;
                    let bh = if secs > 0 { ((secs / 60 * ah as u32 / (max / 60).max(1)) as i32).clamp(2, ah) } else { 0 };
                    dsn_fill(cx + 1, base - bh, (cw - 2).max(1), bh, BLACK);
                    if k % 6 == 0 {
                        text(cx, base + 4, 6 * cw, ls, S::new().n(k as u32).s(b":00").get(), SMALL, LEFT, 0);
                    }
                }
                dsn_line(left, base, left + 24 * cw, base, 1, DARK);
                h
            }
            4 => {
                let n = st.nbooks.min(3);
                let row = lb + 4 + 6 + 10;
                let h = self.head() + n as i32 * row + PAD - 10;
                if n == 0 || !draw {
                    return if n == 0 { 0 } else { h };
                }
                self.card(x, y, w, h, b"Most read");
                let mut picked = [usize::MAX; 3];
                for k in 0..n {
                    let mut best = usize::MAX;
                    for j in 0..st.nbooks {
                        if !picked[..k].contains(&j) && (best == usize::MAX || st.books[j].secs > st.books[best].secs) {
                            best = j;
                        }
                    }
                    picked[k] = best;
                    let b = &st.books[best];
                    let ry = top + k as i32 * row;
                    let time = dur(b.secs);
                    let tw = text_w(time.get(), SMALL, 0) + 8;
                    let title: &[u8] = if b.tlen > 0 { &b.title[..b.tlen] } else { b"Untitled" };
                    text(x + PAD, ry, inner - tw, lb, title, BODY, LEFT, 0);
                    text(x + w - PAD - tw, ry, tw, lb, time.get(), SMALL, RIGHT, 0);
                    let top_secs = st.books[picked[0]].secs;
                    let bw = ((b.secs / 60) * inner as u32 / (top_secs / 60).max(1)) as i32;
                    dsn_fill(x + PAD, ry + lb + 4, inner, 6, LIGHT);
                    dsn_fill(x + PAD, ry + lb + 4, bw.clamp(2, inner), 6, BLACK);
                }
                h
            }
            _ => {
                let (_, best) = st.streaks();
                let tenths = st.progress_bp / 1000; // 10000 bp = one whole book
                let t: [(&[u8], S); 4] = [
                    (b"Sessions", S::new().n(st.sessions)),
                    (b"Average session", dur(st.total / st.sessions.max(1))),
                    (b"Best streak", days(best)),
                    (b"Books' worth read", S::new().n(tenths / 10).s(b".").n(tenths % 10)),
                ];
                self.tiles(y, draw, t)
            }
        }
    }

    fn top(&self) -> i32 {
        STATUS_Y + self.ls + 16 // just the status line above the cards
    }

    // Smallest book card: room for the text block or a 92px cover.
    fn book_min(&self) -> i32 {
        2 * PAD + (self.ls + self.lb + self.ls + 18).max(92)
    }

    // The card is pinned to the bottom of page 1 and grows into whatever the
    // widgets above (ending at `end`) leave free, so the cover can be large.
    fn book_rect(&self, end: i32) -> (i32, i32) {
        let bottom = self.h - PAD;
        let h = (bottom - end).clamp(self.book_min(), 2 * PAD + COVER_MAX);
        (bottom - h, h)
    }

    fn book(&self, home: &Home) {
        let (x, w, (y, h)) = (G, self.w - 2 * G, home.card);
        let (ls, lb) = (self.ls, self.lb);
        dsn_stroke(x, y, w, h, BLACK, 2, 8, 15);
        let mut tx = x + PAD;
        if home.cover_n > 4 {
            let c = &home.cover;
            let (cw, ch) = (c[0] as i32 | (c[1] as i32) << 8, c[2] as i32 | (c[3] as i32) << 8);
            let cy = y + (h - ch) / 2;
            dsn_bitmap(tx, cy, cw, ch, &c[4..home.cover_n], cw, ch, 1, 0, BLACK, 0); // BW, centered
            dsn_stroke(tx, cy, cw, ch, BLACK, 1, 0, 0);
            tx += cw + 16;
        }
        let tw = x + w - PAD - tx;
        let ty = y + (h - (ls + lb + ls + 18)) / 2;
        text(tx, ty, tw, ls, b"Continue reading", SMALL, LEFT, 0);
        text(tx, ty + ls, tw, lb, home.field(1), BODY, LEFT, BOLD);
        text(tx, ty + ls + lb, tw, ls, home.field(2), SMALL, LEFT, 0);
        if home.percent >= 0 {
            let label = S::new().n(home.percent as u32).s(b"%");
            let pw = text_w(b"100%", SMALL, 0) + 10;
            let (by, bw) = (ty + ls + lb + ls + 8, tw - pw);
            dsn_fill_round(tx, by, bw, 10, LIGHT, 5, 15);
            if home.percent > 0 {
                dsn_fill_round(tx, by, (bw * home.percent / 100).max(10), 10, BLACK, 5, 15);
            }
            text(tx + bw, by + 5 - ls / 2, pw, ls, label.get(), SMALL, RIGHT, 0);
        }
    }

    // App grid, pinned to the bottom of every page after the first: (columns, cell width, cell height, top).
    fn grid(&self) -> (i32, i32, i32, i32) {
        let cols = if self.w - 2 * G >= 640 { NAPPS } else { 3 };
        let ch = TILE + 6 + self.ls + GAP;
        (cols, (self.w - 2 * G) / cols, ch, self.h - PAD - (NAPPS / cols) * ch + GAP)
    }

    fn apps(&self, sel: i32) {
        let (cols, cw, ch, top) = self.grid();
        for (i, (label, _)) in APPS.iter().enumerate() {
            let (cx, y) = (G + i as i32 % cols * cw, top + i as i32 / cols * ch);
            let x = cx + (cw - TILE) / 2;
            let on = sel == i as i32;
            dsn_bitmap(x, y, TILE, TILE, &TILES[i], TILE, TILE, 1, 0, BLACK, 0); // BW, centered
            if on {
                dsn_stroke(x - 6, y - 6, TILE + 12, TILE + 12, BLACK, 3, 24, 15);
            }
            text(cx, y + TILE + 6, cw, self.ls, label, SMALL, CENTER, if on { BOLD } else { 0 });
        }
    }

    // The app icon under a tap on page 2, if any.
    fn app_at(&self, x: i32, y: i32) -> Option<usize> {
        let (cols, cw, ch, top) = self.grid();
        if x < G || y < top {
            return None;
        }
        let (c, r) = ((x - G) / cw, (y - top) / ch);
        if c < cols && r < NAPPS / cols { Some((r * cols + c) as usize) } else { None }
    }

    // Lays the stat widgets out over pages, leaving room for the book card on
    // page 1 and the app grid on the others; draws those on `page` when `draw`.
    // Returns (page count, y just below the last widget on `page`).
    fn flow(&self, st: &Stats, home: &Home, page: i32, draw: bool) -> (i32, i32) {
        let top = self.top();
        let foot = |p: i32| match p {
            0 if home.has_book() => self.book_min() + GAP,
            _ => self.h - PAD - self.grid().3 + GAP,
        };
        let (mut p, mut y, mut end) = (0, top, top);
        for i in 0..WIDGETS {
            let h = if st.sessions > 0 { self.widget(st, i, 0, false) } else { 0 };
            if h == 0 {
                continue;
            }
            if y > top && y + h > self.h - PAD - foot(p) {
                p += 1;
                y = top;
            }
            if p == page {
                if draw {
                    self.widget(st, i, y, true);
                }
                end = y + h + GAP;
            }
            y += h + GAP;
        }
        ((p + 1).max(2), end)
    }

    // Draws `page` with app icon `sel` focused (-1 for none); returns the page count.
    fn render(&self, st: &Stats, home: &Home, page: i32, sel: i32) -> i32 {
        dsn_clear();
        let (pages, _) = self.flow(st, home, page, true);
        if st.sessions == 0 && page == 0 {
            let msg: &[u8] = if st.err == ERR_UNAVAILABLE {
                b"Reading stats need newer firmware."
            } else {
                b"No reading sessions yet. Read a book and come back."
            };
            dsn_text_box(G, self.top(), self.w - 2 * G, 4 * self.lb, msg, BODY, CENTER, 4, 0);
        }
        if page == 0 && home.has_book() {
            self.book(home);
        }
        if page > 0 {
            self.apps(sel);
        }
        // Status line (a home screen draws its own): clock, page, battery.
        let (x, w, ls) = (G, self.w - 2 * G, self.ls);
        let mut c = [0u8; 8]; // dsn_clock_t: hour and minute are bytes 4 and 5
        if dsn_clock(&mut c) >= 0 && c[0] | c[1] != 0 {
            let h = if c[4] % 12 == 0 { 12 } else { c[4] % 12 };
            let t = S::new().n(h as u32).s(b":").n(c[5] as u32 / 10).n(c[5] as u32 % 10);
            text(x, STATUS_Y, w / 3, ls, t.s(if c[4] < 12 { b" AM" } else { b" PM" }).get(), SMALL, LEFT, 0);
        }
        text(x + w / 3, STATUS_Y, w / 3, ls, S::new().n(page as u32 + 1).s(b"/").n(pages as u32).get(), SMALL, CENTER, 0);
        let b = dsn_battery(); // percent in the low byte, power state above it
        if b >= 0 {
            text(x + w - w / 3, STATUS_Y, w / 3, ls, S::new().n(b as u32 & 0xff).s(b"%").get(), SMALL, RIGHT, 0);
        }
        pages
    }
}

// The focus NEXT/PREV step through: stop 0 is page 1 (the book card), stops
// 1..=NAPPS the icons on page 2, and one stop per page after that.
fn page_of(stop: i32) -> i32 {
    match stop {
        0 => 0,
        s if s <= NAPPS => 1,
        s => s - NAPPS + 1,
    }
}

fn first_stop(page: i32) -> i32 {
    match page {
        0 => 0,
        1 => 1,
        p => p + NAPPS - 1,
    }
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut st = Stats {
        end: 0,
        day: [0; DAYS],
        hour: [0; 24],
        books: [Book { key: 0, secs: 0, title: [0; 80], tlen: 0 }; NBOOKS],
        nbooks: 0,
        total: 0,
        sessions: 0,
        progress_bp: 0,
        err: 0,
    };
    // dsn_clock_t: u16 year, month, day, ... in local time.
    let mut c = [0u8; 8];
    let year = if dsn_clock(&mut c) >= 0 { c[0] as u32 | (c[1] as u32) << 8 } else { 0 };
    let have_clock = year >= 2000 && (1..=12).contains(&c[2]) && (1..=31).contains(&c[3]);
    if have_clock {
        st.end = days_from_civil(year, c[2] as u32, c[3] as u32);
    }
    st.load(have_clock);

    let scr = dsn_screen();
    let ui = Ui {
        w: (scr >> 16) as i32,
        h: (scr & 0xffff) as i32,
        ls: dsn_line_height(SMALL),
        lb: dsn_line_height(BODY),
        lt: dsn_line_height(TITLE),
    };
    let mut info = [0u8; 24]; // dsn_device_info: has_buttons is byte 19
    dsn_device(&mut info);
    let mut home = Home {
        rec: [0; 1100],
        f: [(0, 0); 3],
        percent: -1,
        cover: [0; 8192],
        cover_n: 0,
        card: (0, 0),
        ring: info[19] != 0,
    };
    home.load();
    if home.has_book() {
        home.card = ui.book_rect(ui.flow(&st, &home, 0, false).1);
        // The cover is as tall as the card allows. Slow the first time (decoded, then cached).
        let n = dsn_book_cover(&home.rec[home.f[0].0..home.f[0].1], home.card.1 - 2 * PAD, &mut home.cover);
        home.cover_n = if n > 4 { (n as usize).min(home.cover.len()) } else { 0 };
    }

    let open = |stop: i32| match stop {
        0 if home.has_book() => dsn_open_book(home.field(0)),
        s if (1..=NAPPS).contains(&s) => dsn_open_home(APPS[s as usize - 1].1),
        _ => 0,
    };
    let mut stop = 0;
    loop {
        let page = page_of(stop);
        let sel = if home.ring && page == 1 { stop - 1 } else { -1 };
        let pages = ui.render(&st, &home, page, sel);
        dsn_refresh();
        let last = first_stop(pages - 1).max(NAPPS); // page 2 is always there, and ends at its last icon
        stop = loop {
            let e = dsn_wait_event();
            // sdk/dessen_abi.h DSN_EV_*; open_* end the app when they succeed.
            let next = match e & 0xff {
                1 => {
                    open(stop); // CONFIRM
                    stop
                }
                2 if stop == 0 => return 0,                       // BACK on page 1: leave
                2 => 0,                                           // BACK elsewhere: page 1
                3 => (stop - 1).max(0),                           // PREV
                4 => (stop + 1).min(last),                        // NEXT
                7 | 9 => first_stop((page + 1).min(pages - 1)),   // swipe up / left
                8 | 10 => first_stop((page - 1).max(0)),          // swipe down / right
                5 => {
                    let (x, y) = ((e >> 8 & 0xfff) as i32, (e >> 20 & 0xfff) as i32);
                    let app = if page > 0 { ui.app_at(x, y) } else { None };
                    if let Some(i) = app {
                        open(i as i32 + 1);
                        stop
                    } else if page == 0 && home.has_book() && y >= home.card.0 {
                        open(0);
                        stop
                    } else {
                        first_stop((page + 1) % pages) // tap on empty space: next page
                    }
                }
                _ => stop,
            };
            if next != stop {
                break next;
            }
        };
    }
}
