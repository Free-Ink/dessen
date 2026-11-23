//! The recents shelf: recently opened books, then recently added ones.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// One book on the shelf.
pub struct Book {
    /// Where it is on the SD card, to open it.
    pub path: String,
    /// Its title.
    pub title: String,
    /// Its author; empty when unknown.
    pub author: String,
    /// How much of it is read, 0 to 100; `None` when unknown.
    pub percent: Option<u32>,
}

/// The books `start..start + count` of the shelf, or `None` when there are
/// none there. Firmware without the shelf answers its recents instead, for
/// the first page.
pub fn page(start: u32, count: usize) -> Option<Vec<Book>> {
    let count = count.clamp(1, dessen::DSN_SHELF_PAGE_MAX as usize);
    let mut records = vec![0u8; count * dessen::DSN_SHELF_RECORD_MAX as usize];
    let mut len = dessen::dsn_recent_shelf(start, count as i32, &mut records);
    if len < 0 && start == 0 {
        len = dessen::dsn_recents(&mut records);
    }
    let len = usize::try_from(len)
        .ok()
        .filter(|len| *len > 0)?
        .min(records.len());
    let books: Vec<Book> = records[..len]
        .split(|byte| *byte == b'\n')
        .filter_map(book)
        .take(count)
        .collect();
    (!books.is_empty()).then_some(books)
}

/// The book in one "path\ttitle\tauthor\t..." record; `None` without a path
/// or a title.
fn book(record: &[u8]) -> Option<Book> {
    let mut fields = record.split(|byte| *byte == b'\t').map(text);
    let path = fields.next()?;
    let title = fields.next()?;
    let author = fields.next().unwrap_or_default();
    if !path.starts_with('/') {
        return None;
    }
    let percent = u32::try_from(dessen::dsn_book_progress(path.as_bytes()))
        .ok()
        .filter(|percent| *percent <= 100);
    Some(Book {
        path,
        title,
        author,
        percent,
    })
}

/// A field as text, up to its first byte that is not UTF-8. The firmware
/// sends UTF-8; this keeps a damaged title readable without the lossy
/// decoder, which costs its weight in code.
fn text(field: &[u8]) -> String {
    let valid = match core::str::from_utf8(field) {
        Ok(text) => text,
        Err(error) => core::str::from_utf8(&field[..error.valid_up_to()]).unwrap_or_default(),
    };
    String::from(valid)
}
