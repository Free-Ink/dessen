//! The screen: the entries as a list, under the clock and battery.

use alloc::string::String;
use alloc::vec::Vec;
use xpui::{Button, List, ListRow, NavigationScreen, Screen, View};

use crate::menu::{self, MENU, Open};

#[derive(Clone, Copy)]
pub enum Msg {
    Open(usize),
    /// Back, claimed so that it does not leave the device's root screen.
    Stay,
}

pub struct Home {
    entries: Vec<(&'static str, Open)>,
    book: Option<String>,
    status: String,
}

impl Home {
    pub fn new() -> Self {
        let book = menu::current_book();
        let mut entries = Vec::new();
        if book.is_some() {
            entries.push(("Continue Reading", Open::Book));
        }

        entries.extend(MENU);

        Home {
            entries,
            book,
            status: kit::status(None),
        }
    }
}

impl Screen for Home {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        let rows = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, (label, _))| ListRow::new(*label).on_tap(Msg::Open(index)));

        NavigationScreen::new(List::new().extend(rows)).title(self.status.clone())
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        (key == Button::Back).then_some(Msg::Stay)
    }

    fn update(&mut self, message: Msg) {
        if let Msg::Open(index) = message
            && let Some((_, entry)) = self.entries.get(index)
        {
            menu::open(*entry, self.book.as_deref());
        }
    }
}
