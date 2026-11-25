use crate::menu::{self, MENU, Open};
use dessen::{reset, with};

#[test]
fn the_menu_opens_firmware_screens_and_an_app() {
    let names: Vec<_> = MENU.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["Library", "Files", "Dice", "Settings"]);
    reset(|_| {});
    for (_, entry) in MENU {
        menu::open(entry, None);
    }
    let want = [
        format!("home {}", dessen::DSN_HOME_LIBRARY),
        format!("home {}", dessen::DSN_HOME_FILES),
        "app /plugins/dice/app.bin".into(),
        format!("home {}", dessen::DSN_HOME_SETTINGS),
    ];
    assert_eq!(with(|w| w.opened.clone()), want);
}

#[test]
fn continue_reading_opens_the_book_and_nothing_without_one() {
    reset(|_| {});
    menu::open(Open::Book, Some("/books/dune.epub"));
    menu::open(Open::Book, None);
    assert_eq!(with(|w| w.opened.clone()), ["book /books/dune.epub"]);
}

#[test]
fn the_current_book_is_the_first_recent() {
    reset(|w| w.recents = Some(b"/books/dune.epub\tDune\tFrank Herbert\n/books/h.epub\tHyperion\t\n".to_vec()));
    assert_eq!(menu::current_book().as_deref(), Some("/books/dune.epub"));
    reset(|w| w.recents = Some(b"/only-a-path".to_vec()));
    assert_eq!(menu::current_book().as_deref(), Some("/only-a-path"));
}

#[test]
fn no_current_book_without_a_path() {
    for recents in [None, Some(Vec::new()), Some(b"dune.epub\tDune\n".to_vec()), Some(b"\xff\xfe\tx\n".to_vec())] {
        reset(|w| w.recents = recents.clone());
        assert_eq!(menu::current_book(), None, "{recents:?}");
    }
}
