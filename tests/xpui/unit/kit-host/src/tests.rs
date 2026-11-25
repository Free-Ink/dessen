use crate::shelf;
use crate::{Padded, battery, clock, join, status};
use dessen::{clock_bytes, reset};

#[test]
fn join_puts_words_and_numbers_side_by_side() {
    assert_eq!(join!("Level ", 3u32, "   Moves ", 12u32), "Level 3   Moves 12");
    assert_eq!(join!(0u32), "0");
    assert_eq!(join!(u32::MAX), "4294967295");
    assert_eq!(join!(7usize, 'x'), "7x");
    assert_eq!(join!(-42i32, " ", i32::MIN), "-42 -2147483648");
    assert_eq!(join!(String::from("a"), "b"), "ab");
}

#[test]
fn padded_fills_with_zeros_but_never_cuts() {
    assert_eq!(join!(9u32, ":", Padded(5, 2)), "9:05");
    assert_eq!(join!(Padded(0, 2)), "00");
    assert_eq!(join!(Padded(123, 2)), "123");
    assert_eq!(join!(Padded(7, 0)), "7");
    assert_eq!(join!(Padded(42, 12)), "0000000042"); // ten digits at most
}

#[test]
fn clock_reads_twelve_hour_time() {
    let at = |hour, minute| {
        reset(|w| w.clock = Some(clock_bytes(2025, 11, 24, hour, minute, 0)));
        clock()
    };
    assert_eq!(at(0, 0).as_deref(), Some("12:00 AM"));
    assert_eq!(at(9, 5).as_deref(), Some("9:05 AM"));
    assert_eq!(at(12, 30).as_deref(), Some("12:30 PM"));
    assert_eq!(at(23, 59).as_deref(), Some("11:59 PM"));
}

#[test]
fn clock_is_none_when_unset_or_missing() {
    reset(|w| w.clock = Some(clock_bytes(0, 1, 1, 10, 0, 0)));
    assert_eq!(clock(), None);
    reset(|w| w.clock = None);
    assert_eq!(clock(), None);
}

#[test]
fn battery_is_the_low_byte() {
    reset(|w| w.battery = 0x0100 | 87); // charging, 87%
    assert_eq!(battery().as_deref(), Some("87%"));
    reset(|w| w.battery = -4);
    assert_eq!(battery(), None);
}

#[test]
fn status_joins_what_the_board_has() {
    reset(|w| {
        w.clock = Some(clock_bytes(2025, 11, 24, 16, 0, 0));
        w.battery = 72;
    });
    assert_eq!(status(None), "4:00 PM  72%");
    assert_eq!(status(Some("1/2")), "4:00 PM  1/2  72%");
    reset(|_| {});
    assert_eq!(status(None), "Home");
    assert_eq!(status(Some("Recents")), "Recents");
}

fn record(path: &str, title: &str, author: &str) -> Vec<u8> {
    format!("{path}\t{title}\t{author}\t").into_bytes()
}

#[test]
fn shelf_reads_records_with_their_progress() {
    reset(|w| {
        w.shelf = Some(vec![record("/books/dune.epub", "Dune", "Frank Herbert"), record("/b/h.epub", "Hyperion", "")]);
        w.progress.insert(b"/books/dune.epub".to_vec(), 37);
    });
    let books = shelf::page(0, 8).expect("books");
    assert_eq!(books.len(), 2);
    assert_eq!((books[0].path.as_str(), books[0].title.as_str()), ("/books/dune.epub", "Dune"));
    assert_eq!(books[0].author, "Frank Herbert");
    assert_eq!(books[0].percent, Some(37));
    assert_eq!(books[1].author, "");
    assert_eq!(books[1].percent, None); // unknown progress
}

#[test]
fn shelf_pages_and_ends() {
    reset(|w| w.shelf = Some((0..5).map(|n| record(&format!("/b/{n}"), &format!("Book {n}"), "")).collect()));
    let second = shelf::page(2, 2).expect("a second page");
    assert_eq!(second.iter().map(|b| b.title.as_str()).collect::<Vec<_>>(), ["Book 2", "Book 3"]);
    assert_eq!(shelf::page(4, 2).expect("the last book").len(), 1);
    assert!(shelf::page(5, 2).is_none());
}

#[test]
fn shelf_skips_broken_records() {
    reset(|w| {
        w.shelf = Some(vec![
            b"no-slash\tTitle\t".to_vec(), // not a path
            b"/only-a-path".to_vec(),      // no title
            record("/ok", "Kept", ""),
            b"/bad\tTi\xfftle\t".to_vec(), // cut at the first byte that is not UTF-8
        ]);
        w.progress.insert(b"/ok".to_vec(), 101); // out of range: unknown
    });
    let books = shelf::page(0, 8).expect("books");
    assert_eq!(books.iter().map(|b| b.title.as_str()).collect::<Vec<_>>(), ["Kept", "Ti"]);
    assert_eq!(books[0].percent, None);
}

#[test]
fn shelf_falls_back_to_recents_on_old_firmware() {
    reset(|w| {
        w.shelf = None;
        w.recents = Some(b"/books/dune.epub\tDune\tFrank Herbert\n".to_vec());
    });
    assert_eq!(shelf::page(0, 3).expect("recents")[0].title, "Dune");
    assert!(shelf::page(3, 3).is_none(), "recents only stand in for the first page");
    reset(|_| {});
    assert!(shelf::page(0, 3).is_none());
}
