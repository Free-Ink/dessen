//! The reading session log, tallied: time per day and per hour, totals, and
//! time per book.

use crate::json;

/// Where the firmware keeps the log.
pub const LOG: &[u8] = b"/.crosspoint/reading_sessions.jsonl";
/// Days of history kept, for the heatmap: 53 weeks, newest last.
pub const DAYS: usize = 53 * 7;
/// Distinct books tallied for "Most read".
const BOOKS_KEPT: usize = 64;
/// Books "Most read" shows.
const RANKED: usize = 3;
const SECONDS_PER_DAY: i64 = 86400;

/// Whether the log is missing or empty.
pub fn is_empty() -> bool {
    dessen::dsn_fs_read_at(LOG, 0, &mut [0u8; 1]) <= 0
}

/// One book's reading time, as it is tallied.
///
/// Its title stays in the log, found again by offset for the few books that
/// are shown: 64 titles held while tallying were 5 KB of the heap.
#[derive(Clone, Copy)]
struct Tally {
    /// FNV-1a of its sync document id.
    key: u32,
    seconds: u32,
    /// Where its title starts in the log, just inside the quotes;
    /// `NO_TITLE` when its line has none.
    title_at: u32,
}

const NO_TITLE: u32 = u32::MAX;

impl Tally {
    const ZERO: Tally = Tally {
        key: 0,
        seconds: 0,
        title_at: 0,
    };
}

/// A book "Most read" shows: its time and title.
#[derive(Clone, Copy)]
pub struct Book {
    pub seconds: u32,
    title: [u8; 80],
    title_len: usize,
}

impl Book {
    const ZERO: Book = Book {
        seconds: 0,
        title: [0; 80],
        title_len: 0,
    };

    /// A book read for `seconds`, its title read from the log at `title_at`.
    fn read(seconds: u32, title_at: u32) -> Book {
        let mut book = Book {
            seconds,
            title: [0; 80],
            title_len: 0,
        };
        // Room for a whole title even if every byte were escaped.
        let mut raw = [0u8; 160];
        let read = if title_at == NO_TITLE {
            0
        } else {
            dessen::dsn_fs_read_at(LOG, title_at, &mut raw)
        };
        let mut title = raw[..read.clamp(0, raw.len() as i32) as usize].iter();
        while let Some(&byte) = title.next() {
            // JSON string: stop at the closing quote, take an escaped byte literally.
            let byte = match byte {
                b'"' => break,
                b'\\' => *title.next().unwrap_or(&b'?'),
                byte => byte,
            };
            if book.title_len == book.title.len() {
                break;
            }
            book.title[book.title_len] = byte;
            book.title_len += 1;
        }
        book
    }

    pub fn title(&self) -> &str {
        match core::str::from_utf8(&self.title[..self.title_len]) {
            Ok("") | Err(_) => "Untitled",
            Ok(title) => title,
        }
    }
}

pub struct Stats {
    /// The local day number (days since 1970) of `day[DAYS - 1]`: today.
    pub end: u32,
    /// Seconds read per local day.
    pub day: [u32; DAYS],
    /// Seconds by the local hour each session started in.
    pub hour: [u32; 24],
    books: [Tally; BOOKS_KEPT],
    pub book_count: usize,
    /// The most read books, most first: the first `ranked` are filled.
    ranked: [Book; RANKED],
    ranked_count: usize,
    pub total_seconds: u32,
    pub sessions: u32,
    /// Forward book progress, summed, in basis points: 10000 is one book.
    pub progress: u32,
    /// `DSN_ERR_UNAVAILABLE` when the firmware cannot read files; else 0.
    pub error: i32,
}

impl Stats {
    const ZERO: Stats = Stats {
        end: 0,
        day: [0; DAYS],
        hour: [0; 24],
        books: [Tally::ZERO; BOOKS_KEPT],
        book_count: 0,
        ranked: [Book::ZERO; RANKED],
        ranked_count: 0,
        total_seconds: 0,
        sessions: 0,
        progress: 0,
        error: 0,
    };

    /// The app's one record, ending on `today`. At ~2.6 KB it is too big to
    /// build on the stack, and it lives as long as the app, so it is a static
    /// in the image's zeroed memory rather than on the heap.
    ///
    /// # Panics
    /// The second time: there is one record.
    pub fn new(today: u32) -> &'static mut Stats {
        static STATS: dessen::ConstStaticCell<Stats> = dessen::ConstStaticCell::new(Stats::ZERO);
        let stats = STATS.take().expect("one Stats");
        stats.end = today;
        stats
    }

    /// Reads the log again from scratch, in place.
    pub fn reload(&mut self, today: u32, clock_valid: bool) {
        self.end = today;
        self.day.fill(0);
        self.hour.fill(0);
        self.book_count = 0;
        self.total_seconds = 0;
        self.sessions = 0;
        self.progress = 0;
        self.error = 0;
        self.load(clock_valid);
    }

    /// Reads the log: tallies every session, then ranks the books.
    pub fn load(&mut self, clock_valid: bool) {
        self.tally(clock_valid);
        self.rank();
    }

    /// Streams the log a buffer at a time, tallying each whole line.
    fn tally(&mut self, clock_valid: bool) {
        let mut buffer = [0u8; 2048];
        let (mut len, mut offset, mut skipping) = (0usize, 0u32, false);
        loop {
            let read = dessen::dsn_fs_read_at(LOG, offset, &mut buffer[len..]);
            if read <= 0 {
                // A missing log (IO error) just means nothing has been read yet.
                if read == dessen::DSN_ERR_UNAVAILABLE {
                    self.error = read;
                }
                return; // an unfinished last line is left out, as the firmware does
            }
            let read = (read as usize).min(buffer.len() - len);
            offset += read as u32;
            len += read;
            // Where buffer[0] is in the log.
            let base = offset - len as u32;
            let mut start = 0;
            while let Some(newline) = buffer[start..len].iter().position(|byte| *byte == b'\n') {
                if !skipping {
                    let line = &buffer[start..start + newline];
                    self.add(line, base + start as u32, clock_valid);
                }
                skipping = false;
                start += newline + 1;
            }
            buffer.copy_within(start..len, 0);
            len -= start;
            if len == buffer.len() {
                // A line longer than the buffer: drop it through its newline.
                len = 0;
                skipping = true;
            }
        }
    }

    /// Moves the day tallies `days` back, for a new "today".
    fn shift(&mut self, days: u32) {
        let days = (days as usize).min(DAYS);
        self.day.copy_within(days.., 0);
        self.day[DAYS - days..].fill(0);
    }

    /// Tallies one line of the log, which starts at `line_at` in it.
    fn add(&mut self, line: &[u8], line_at: u32, clock_valid: bool) {
        let start = json::number(line, b"\"start\":");
        let seconds = json::number(line, b"\"secs\":");
        // Same sanity limits the firmware applies before uploading a session.
        let plausible_start = (946_684_800..=0xf000_0000).contains(&start);
        if !plausible_start || seconds <= 0 || seconds > 7 * SECONDS_PER_DAY {
            return;
        }
        let seconds = seconds as u32;
        let offset_minutes = json::number(line, b"\"tz\":").clamp(-720, 840);
        let local = (start + offset_minutes * 60) as u32;
        let day = local / SECONDS_PER_DAY as u32;
        if day > self.end {
            if clock_valid {
                return; // dated in the future: measured with a wrong clock
            }
            // No clock: "today" is the newest session seen so far.
            self.shift(day - self.end);
            self.end = day;
        }
        // ponytail: a session counts wholly toward the day and hour it started in;
        // split at midnight/hour boundaries if late-night sessions look wrong.
        let days_ago = (self.end - day) as usize;
        if days_ago < DAYS {
            self.day[DAYS - 1 - days_ago] += seconds;
        }
        self.hour[(local % SECONDS_PER_DAY as u32 / 3600) as usize] += seconds;
        self.total_seconds += seconds;
        self.sessions += 1;
        let from = json::number(line, b"\"from\":").clamp(0, 10000);
        let to = json::number(line, b"\"to\":").clamp(0, 10000);
        if to > from {
            self.progress += (to - from) as u32;
        }
        if let Some(index) = self.book_index(line, line_at) {
            self.books[index].seconds += seconds;
        }
    }

    /// Where the line's book is tallied, adding it when it is new; `None`
    /// without a document id.
    fn book_index(&mut self, line: &[u8], line_at: u32) -> Option<usize> {
        let document = json::after(line, b"\"doc\":\"")?;
        let key = document
            .iter()
            .take_while(|byte| **byte != b'"')
            .fold(0x811c_9dc5u32, |hash, byte| {
                (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
            });
        let known = &self.books[..self.book_count];
        if let Some(index) = known.iter().position(|book| book.key == key) {
            return Some(index);
        }
        // ponytail: past BOOKS_KEPT distinct books the least-read one is dropped,
        // so its earlier time is lost from "Most read" (totals are unaffected).
        let index = if self.book_count < BOOKS_KEPT {
            self.book_count += 1;
            self.book_count - 1
        } else {
            (0..BOOKS_KEPT)
                .min_by_key(|index| self.books[*index].seconds)
                .unwrap_or(0)
        };
        let title_at = json::after(line, b"\"title\":\"").map_or(NO_TITLE, |title| {
            line_at + (title.as_ptr() as usize - line.as_ptr() as usize) as u32
        });
        self.books[index] = Tally {
            key,
            seconds: 0,
            title_at,
        };
        Some(index)
    }

    /// (current, best) runs of consecutive days with reading. Today not having
    /// been read yet doesn't break the current streak.
    pub fn streaks(&self) -> (u32, u32) {
        let (mut best, mut run) = (0, 0);
        for &seconds in self.day.iter() {
            run = if seconds > 0 { run + 1 } else { 0 };
            best = best.max(run);
        }
        let unread_today = usize::from(self.day[DAYS - 1] == 0);
        let current = self
            .day
            .iter()
            .rev()
            .skip(unread_today)
            .take_while(|seconds| **seconds > 0)
            .count() as u32;
        (current, best) // ponytail: "best" only sees the last 53 weeks
    }

    /// Picks the most read books and reads their titles back from the log.
    fn rank(&mut self) {
        let mut picked = [usize::MAX; RANKED];
        let count = RANKED.min(self.book_count);
        for rank in 0..count {
            let unpicked = (0..self.book_count).filter(|index| !picked[..rank].contains(index));
            // Of equal times, the book tallied first ranks higher.
            let best = unpicked.reduce(|best, index| {
                if self.books[index].seconds > self.books[best].seconds {
                    index
                } else {
                    best
                }
            });
            picked[rank] = best.unwrap_or(0);
        }
        for (rank, index) in picked.into_iter().take(count).enumerate() {
            let tally = self.books[index];
            self.ranked[rank] = Book::read(tally.seconds, tally.title_at);
        }
        self.ranked_count = count;
    }

    /// Up to `count` books, most read first.
    pub fn most_read(&self, count: usize) -> impl Iterator<Item = &Book> {
        self.ranked[..self.ranked_count].iter().take(count)
    }
}
