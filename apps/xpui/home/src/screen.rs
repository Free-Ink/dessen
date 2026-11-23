//! The screen: the shelf over the bar, and how input moves through it.

use xpui::host::SwipeDir;
use xpui::{Button, Modifiers, NavigationScreen, Screen, View, vstack};

use crate::nav_bar::{NAV, NavBar};
use crate::recents::Recents;
use crate::shelf_view::ShelfView;

#[derive(Clone, Copy)]
pub enum Msg {
    /// Open book `index` of the page.
    Open(usize),
    Select(usize),
    /// Turn the page: forward when true.
    Page(bool),
    /// Open the firmware screen in bar cell `index`.
    Nav(usize),
    /// Back: open the file browser.
    Browse,
    /// Look for books again, while the shelf is empty.
    Reload,
}

impl Screen for Recents {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        let shelf = ShelfView::new(self.page.clone(), self.selected);
        NavigationScreen::new(vstack![0; shelf.flexible(), NavBar::new()])
            .title(kit::join!("Recents  ", self.range()))
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        if self.is_empty() {
            return Some(match key {
                Button::Back => Msg::Browse,
                _ => Msg::Reload,
            });
        }
        let shown = self.page.books.len();
        Some(match key {
            Button::Back => Msg::Browse,
            Button::Confirm => Msg::Open(self.selected),
            Button::Down if self.selected + 1 < shown => Msg::Select(self.selected + 1),
            Button::Down | Button::Right | Button::PageForward => Msg::Page(true),
            Button::Up if self.selected > 0 => Msg::Select(self.selected - 1),
            Button::Up | Button::Left | Button::PageBack => Msg::Page(false),
            _ => return None,
        })
    }

    fn on_swipe(&self, direction: SwipeDir) -> Option<Msg> {
        match direction {
            SwipeDir::Up | SwipeDir::Left => Some(Msg::Page(true)),
            SwipeDir::Down | SwipeDir::Right => Some(Msg::Page(false)),
            SwipeDir::None => None,
        }
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Open(index) => self.open(index),
            Msg::Select(index) => self.selected = index,
            // With nothing shown, a swipe forward looks for books again.
            Msg::Page(forward) if self.is_empty() => {
                if forward {
                    self.show(0);
                }
            }
            Msg::Page(forward) => self.turn(forward),
            Msg::Nav(index) => {
                dessen::dsn_open_home(NAV[index].1);
            }
            Msg::Browse => {
                // Touch boards have no Back key; there the bar is the way out.
                dessen::dsn_open_browser();
            }
            Msg::Reload => {
                self.show(0);
            }
        }
    }
}
