use crate::bytes::Bytes;
use crate::clock::Clock;
use crate::log::{DAYS, LOG, Stats};
use crate::{format, json, log};
use dessen::{clock_bytes, reset, with};

// ---- json ----

#[test]
fn json_number_reads_signed_integers() {
    let line = br#"{"start":1764000000,"tz":-60,"secs":0,"big":12345678901234567}"#;
    assert_eq!(json::number(line, b"\"start\":"), 1_764_000_000);
    assert_eq!(json::number(line, b"\"tz\":"), -60);
    assert_eq!(json::number(line, b"\"secs\":"), 0);
    assert_eq!(json::number(line, b"\"missing\":"), 0);
    assert_eq!(json::number(line, b"\"big\":"), 123_456_789_012, "twelve digits at most");
}

#[test]
fn json_string_unescapes_and_refuses_what_does_not_fit() {
    let mut out = [0u8; 32];
    let line = br#"{"title": "The \"Left\" Hand","path":"C:\\x","open":"never"#;
    let len = json::string(line, b"\"title\":", &mut out).expect("title");
    assert_eq!(&out[..len], br#"The "Left" Hand"#);
    let len = json::string(line, b"\"path\":", &mut out).expect("path");
    assert_eq!(&out[..len], br"C:\x");
    assert_eq!(json::string(line, b"\"open\":", &mut out), None, "unfinished");
    assert_eq!(json::string(line, b"\"none\":", &mut out), None, "missing");
    assert_eq!(json::string(br#"{"n":5}"#, b"\"n\":", &mut out), None, "not a string");
    assert_eq!(json::string(br#"{"t":"abcdef"}"#, b"\"t\":", &mut [0u8; 5]), None, "longer than out");
}

// ---- bytes ----

#[test]
fn bytes_refuses_a_piece_that_does_not_fit_whole() {
    let mut bytes = Bytes::<8>::new();
    assert!(bytes.push(b"abc"));
    assert!(!bytes.push(b"defghi"));
    assert_eq!(bytes.as_slice(), b"abc");
    assert!(!bytes.push_all(&[b"de", b"fgh", b"i"]));
    assert_eq!(bytes.as_slice(), b"abcdefgh", "the pieces before the one that did not fit stay");
    bytes.trim_end(b'h');
    assert_eq!(bytes.as_slice(), b"abcdefg");
}

#[test]
fn bytes_trims_every_trailing_match() {
    let mut bytes = Bytes::<8>::new();
    bytes.push(b"a//");
    bytes.trim_end(b'/');
    assert_eq!(bytes.as_slice(), b"a");
    bytes.trim_end(b'a');
    assert_eq!(bytes.as_slice(), b"");
}

// ---- format ----

#[test]
fn durations_read_as_hours_and_minutes() {
    assert_eq!(format::duration(0), "0m");
    assert_eq!(format::duration(59), "0m");
    assert_eq!(format::duration(45 * 60), "45m");
    assert_eq!(format::duration(3600), "1h 0m");
    assert_eq!(format::duration(5 * 3600 + 45 * 60), "5h 45m");
    assert_eq!(format::duration(99 * 3600 + 59 * 60), "99h 59m");
    assert_eq!(format::duration(100 * 3600 + 59 * 60), "100h");
}

#[test]
fn days_agree_in_number() {
    assert_eq!(format::days(0), "0 days");
    assert_eq!(format::days(1), "1 day");
    assert_eq!(format::days(12), "12 days");
}

// ---- clock ----

/// 2025-11-24 16:00:00 UTC, the trace goldens' "now".
const NOW: u32 = 1_764_000_000;
const TODAY: u32 = NOW / 86400;

#[test]
fn clock_gives_today_and_no_offset_in_utc() {
    reset(|w| {
        w.clock = Some(clock_bytes(2025, 11, 24, 16, 0, 0));
        w.time = NOW;
    });
    let clock = Clock::read();
    assert!(clock.valid);
    assert_eq!(clock.today, TODAY);
    assert_eq!(clock.offset_minutes, 0);
}

#[test]
fn clock_offset_is_minutes_behind_utc() {
    // Local 17:00 at 16:00 UTC: UTC+1, which JavaScript calls -60.
    reset(|w| {
        w.clock = Some(clock_bytes(2025, 11, 24, 17, 0, 0));
        w.time = NOW;
    });
    assert_eq!(Clock::read().offset_minutes, -60);
    // Local 10:30:07 at 16:00:07 UTC: UTC-5:30. The time is read just after
    // the clock, so it can be a second on, never behind.
    for late in [0, 1] {
        reset(|w| {
            w.clock = Some(clock_bytes(2025, 11, 24, 10, 30, 7));
            w.time = NOW + 7 + late;
        });
        assert_eq!(Clock::read().offset_minutes, 330);
    }
    // Still the 23rd locally at 08:00 UTC on the 24th (UTC-9): today is the local day.
    reset(|w| {
        w.clock = Some(clock_bytes(2025, 11, 23, 23, 0, 0));
        w.time = NOW - 8 * 3600;
    });
    let clock = Clock::read();
    assert_eq!((clock.today, clock.offset_minutes), (TODAY - 1, 9 * 60));
    // Absurd differences are clamped to real zones.
    reset(|w| {
        w.clock = Some(clock_bytes(2025, 11, 20, 0, 0, 0));
        w.time = NOW;
    });
    assert_eq!(Clock::read().offset_minutes, 720);
}

#[test]
fn clock_counts_leap_days() {
    reset(|w| w.clock = Some(clock_bytes(2024, 2, 29, 0, 0, 0)));
    assert_eq!(Clock::read().today, 19782);
    reset(|w| w.clock = Some(clock_bytes(2024, 3, 1, 0, 0, 0)));
    assert_eq!(Clock::read().today, 19783);
    reset(|w| w.clock = Some(clock_bytes(2000, 1, 1, 0, 0, 0)));
    assert_eq!(Clock::read().today, 10957);
}

#[test]
fn clock_without_a_date_is_not_valid() {
    for clock in [None, Some(clock_bytes(1999, 12, 31, 0, 0, 0)), Some(clock_bytes(2025, 13, 1, 0, 0, 0)), Some(clock_bytes(2025, 1, 0, 0, 0, 0))] {
        reset(|w| {
            w.clock = clock;
            w.time = NOW;
        });
        let read = Clock::read();
        assert!(!read.valid, "{clock:?}");
        assert_eq!((read.today, read.offset_minutes), (0, 0));
    }
}

// ---- the log ----

const DUNE: &str = "dddddddddddddddddddddddddddddddd";
const HAND: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn session(doc: &str, title: &str, days_ago: i64, secs: i64) -> String {
    let start = NOW as i64 - days_ago * 86400;
    format!(
        r#"{{"doc":"{doc}","start":{start},"secs":{secs},"tz":0,"from":1000,"to":1500,"title":"{title}","path":"/books/x.epub"}}"#
    )
}

fn write_log(lines: &[String]) {
    let mut text = lines.join("\n");
    text.push('\n');
    with(|w| w.files.as_mut().unwrap().insert(LOG.to_vec(), text.into_bytes()));
}

/// The log tools/readstats_fixture.py writes, which the goldens draw.
fn fixture() -> Vec<String> {
    let mut lines: Vec<String> = (10..40).rev().map(|d| session(DUNE, "Dune", d, 600)).collect();
    lines.push("not json".into());
    lines.push("x".repeat(3000));
    lines.push(session(HAND, r#"The \"Left\" Hand"#, 1, 900));
    lines.push(session(DUNE, "Dune", 0, 1800));
    lines.push(session(DUNE, "Dune", -3, 5000));
    lines
}

fn load(today: u32, clock_valid: bool) -> &'static mut Stats {
    let stats = Stats::new(today);
    stats.load(clock_valid);
    stats
}

#[test]
fn the_fixture_adds_up_to_the_cards() {
    reset(|_| {});
    write_log(&fixture());
    let stats = load(TODAY, true);
    assert_eq!(stats.error, 0);
    assert_eq!(stats.sessions, 32);
    assert_eq!(format::duration(stats.total_seconds), "5h 45m");
    assert_eq!(stats.day[DAYS - 1], 1800, "today");
    assert_eq!(stats.day[DAYS - 2], 900, "yesterday");
    assert_eq!(stats.day[DAYS - 8..].iter().sum::<u32>(), 45 * 60, "the last 7 days");
    assert_eq!(stats.streaks(), (2, 30));
    assert_eq!(stats.hour[16], stats.total_seconds, "every session started at 16:00");
    assert_eq!(stats.progress, 32 * 500, "1.6 books' worth");
    let most: Vec<_> = stats.most_read(3).map(|b| (b.title().to_owned(), b.seconds)).collect();
    assert_eq!(most, [("Dune".to_owned(), 19800), (r#"The "Left" Hand"#.to_owned(), 900)]);
}

#[test]
fn a_missing_log_is_nothing_read_yet() {
    reset(|_| {});
    assert!(log::is_empty());
    let stats = load(TODAY, true);
    assert_eq!((stats.sessions, stats.error), (0, 0));
    assert_eq!(stats.streaks(), (0, 0));
    assert_eq!(stats.most_read(3).count(), 0);
}

#[test]
fn a_board_without_files_says_so() {
    reset(|w| w.files = None);
    let stats = load(TODAY, true);
    assert_eq!(stats.error, dessen::DSN_ERR_UNAVAILABLE);
}

#[test]
fn implausible_sessions_are_left_out() {
    reset(|_| {});
    write_log(&[
        session(DUNE, "Dune", 0, 0),          // no time
        session(DUNE, "Dune", 0, -5),         // negative
        session(DUNE, "Dune", 0, 7 * 86400 + 1), // longer than a week
        r#"{"doc":"x","start":100,"secs":60}"#.into(), // before 2000
        session(DUNE, "Dune", 0, 60),
    ]);
    let stats = load(TODAY, true);
    assert_eq!((stats.sessions, stats.total_seconds), (1, 60));
}

#[test]
fn an_unfinished_last_line_is_left_out() {
    reset(|_| {});
    let text = format!("{}\n{}", session(DUNE, "Dune", 0, 60), session(DUNE, "Dune", 0, 120));
    with(|w| w.files.as_mut().unwrap().insert(LOG.to_vec(), text.into_bytes()));
    assert_eq!(load(TODAY, true).total_seconds, 60);
}

#[test]
fn a_long_log_streams_through_the_buffer() {
    reset(|_| {});
    // Far more than the 2 KB buffer, with over-long lines among them.
    let mut lines = Vec::new();
    for day in 0..300 {
        lines.push(session(DUNE, "Dune", day, 60));
        if day % 50 == 0 {
            lines.push("y".repeat(5000));
        }
    }
    write_log(&lines);
    let stats = load(TODAY, true);
    assert_eq!(stats.sessions, 300);
    assert_eq!(stats.streaks(), (300, 300));
}

#[test]
fn the_time_zone_moves_a_session_to_its_local_day() {
    reset(|_| {});
    // 23:30 UTC yesterday, in UTC+1: 00:30 today.
    let start = (TODAY as i64) * 86400 - 1800;
    write_log(&[format!(r#"{{"doc":"d","start":{start},"secs":60,"tz":60}}"#)]);
    let stats = load(TODAY, true);
    assert_eq!(stats.day[DAYS - 1], 60);
    assert_eq!(stats.hour[0], 60);
}

#[test]
fn without_a_clock_today_is_the_newest_session() {
    reset(|_| {});
    write_log(&[session(DUNE, "Dune", 5, 60), session(DUNE, "Dune", -3, 120)]);
    let stats = load(0, false);
    assert_eq!(stats.end, TODAY + 3);
    assert_eq!(stats.day[DAYS - 1], 120);
    assert_eq!(stats.day[DAYS - 1 - 8], 60);
    assert_eq!(stats.sessions, 2);
}

#[test]
fn today_unread_does_not_break_the_streak() {
    reset(|_| {});
    write_log(&[session(DUNE, "Dune", 3, 60), session(DUNE, "Dune", 2, 60), session(DUNE, "Dune", 1, 60)]);
    assert_eq!(load(TODAY, true).streaks(), (3, 3));
    write_log(&[session(DUNE, "Dune", 3, 60), session(DUNE, "Dune", 2, 60)]);
    assert_eq!(load(TODAY, true).streaks(), (0, 2), "a day missed ends it");
}

#[test]
fn most_read_ranks_by_time_then_first_seen() {
    reset(|_| {});
    write_log(&[
        session("b1", "First", 0, 100),
        session("b2", "Second", 0, 300),
        session("b3", "Third", 0, 100),
        session("b4", "", 0, 50),
        session("b1", "First", 0, 100),
    ]);
    let stats = load(TODAY, true);
    let most: Vec<_> = stats.most_read(3).map(|b| (b.title().to_owned(), b.seconds)).collect();
    assert_eq!(most, [("Second".into(), 300), ("First".into(), 200), ("Third".into(), 100)]);
    assert_eq!(stats.most_read(1).count(), 1);
    assert_eq!(stats.book_count, 4);
}

#[test]
fn a_book_without_a_title_reads_untitled() {
    reset(|_| {});
    write_log(&[r#"{"doc":"z","start":1764000000,"secs":60}"#.to_string(), session("y", "", 0, 30)]);
    let titles: Vec<_> = load(TODAY, true).most_read(3).map(|b| b.title().to_owned()).collect();
    assert_eq!(titles, ["Untitled", "Untitled"]);
}

#[test]
fn past_64_books_the_least_read_is_dropped() {
    reset(|_| {});
    let mut lines: Vec<String> = (0..64).map(|n| session(&format!("doc{n}"), &format!("Book {n}"), 0, 100 + n)).collect();
    lines.push(session("late", "Late", 0, 1000));
    write_log(&lines);
    let stats = load(TODAY, true);
    assert_eq!(stats.book_count, 64);
    assert_eq!(stats.sessions, 65, "the totals keep every session");
    assert_eq!(stats.most_read(1).next().map(|b| b.title().to_owned()).as_deref(), Some("Late"));
}

#[test]
fn reload_starts_from_scratch() {
    reset(|_| {});
    write_log(&fixture());
    let stats = load(TODAY, true);
    write_log(&[session(DUNE, "Dune", 0, 60)]);
    stats.reload(TODAY, true);
    assert_eq!((stats.sessions, stats.total_seconds, stats.progress), (1, 60, 500));
    assert_eq!(stats.streaks(), (1, 1));
}
