//! What the launcher lists, and opening it.

use alloc::string::String;

/// What an entry opens.
#[derive(Clone, Copy)]
pub enum Open {
    /// A firmware screen, one of `DSN_HOME_*`.
    Builtin(i32),
    /// An installed dessen app, by the path of its `app.bin`.
    App(&'static str),
    /// The most recently read book.
    Book,
}

/// The launcher's entries, top to bottom, after Continue Reading.
pub const MENU: [(&str, Open); 4] = [
    ("Library", Open::Builtin(dessen::DSN_HOME_LIBRARY)),
    ("Files", Open::Builtin(dessen::DSN_HOME_FILES)),
    ("Dice", Open::App("/plugins/dice/app.bin")),
    ("Settings", Open::Builtin(dessen::DSN_HOME_SETTINGS)),
];

/// The most recent book's path, from the first record of the recents list.
pub fn current_book() -> Option<String> {
    let mut list = [0u8; 512];
    let len = usize::try_from(dessen::dsn_recents(&mut list))
        .ok()?
        .min(list.len());
    let first = list[..len]
        .split(|byte| *byte == b'\t' || *byte == b'\n')
        .next()?;
    let path = core::str::from_utf8(first).ok()?;
    path.starts_with('/').then(|| String::from(path))
}

/// Opens `entry`, with `book` as the book to continue. Each call ends the app
/// when it succeeds; the firmware runs this launcher again when the screen or
/// app it opened exits.
pub fn open(entry: Open, book: Option<&str>) {
    match entry {
        Open::Builtin(screen) => {
            dessen::dsn_open_home(screen);
        }
        Open::App(path) => {
            dessen::dsn_open_app(path.as_bytes());
        }
        Open::Book => {
            if let Some(path) = book {
                dessen::dsn_open_book(path.as_bytes());
            }
        }
    }
}
