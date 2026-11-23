//! The screen: which page is shown, the key focus, and opening things.

use alloc::rc::Rc;
use core::cell::RefCell;
use kit::Face;
use xpui::host::{SwipeDir, Theme};
use xpui::{Button, NavigationScreen, Point, Rect, Renderer, Screen, View};

use crate::app_grid::{APP_COUNT, APPS};
use crate::clock::Clock;
use crate::focus::{first_stop, page_of};
use crate::layout::GUTTER;
use crate::model::Model;
use crate::page_view::PageView;
use crate::sync::Account;

#[derive(Clone, Copy)]
pub enum Msg {
    /// Move the focus to a stop; see `focus`.
    Focus(i32),
    /// Open what a focus stop stands for.
    Open(i32),
    Import,
}

pub struct ReadStats {
    model: Rc<RefCell<Model>>,
    stop: i32,
    /// Whether to ring the focused icon: only boards with keys need it.
    focus_ring: bool,
    account: Option<Account>,
    clock: Clock,
}

impl ReadStats {
    pub fn new(model: Model, account: Option<Account>, clock: Clock, focus_ring: bool) -> Self {
        ReadStats {
            model: Rc::new(RefCell::new(model)),
            stop: 0,
            focus_ring,
            account,
            clock,
        }
    }

    fn page(&self) -> i32 {
        page_of(self.stop)
    }

    fn pages(&self) -> i32 {
        self.model.borrow().pages()
    }

    /// The first stop of the page after this one, or the one before.
    fn turn(&self, forward: bool) -> Msg {
        let page = if forward {
            (self.page() + 1).min(self.pages() - 1)
        } else {
            (self.page() - 1).max(0)
        };
        Msg::Focus(first_stop(page))
    }

    /// Opens what focus `stop` stands for. The open calls end the app when
    /// they succeed.
    fn open(&self, stop: i32) {
        let model = self.model.borrow();
        match stop {
            0 => {
                if let Some(book) = &model.book_card.book {
                    dessen::dsn_open_book(book.path.as_bytes());
                }
            }
            stop if (1..=APP_COUNT).contains(&stop) => {
                dessen::dsn_open_home(APPS[stop as usize - 1].1);
            }
            _ => {}
        }
    }

    fn import(&mut self) {
        let Some(account) = &self.account else { return };
        let layout = self.model.borrow().layout;
        let origin = Theme::content_area().origin;
        // The download blocks this screen, so its progress is painted here,
        // outside xpui's frame.
        let show = |message: &str| {
            let area = Rect::new(origin.x, origin.y, layout.width, layout.height);
            Renderer::fill_rect(area, false);
            let top = origin.y + layout.height / 2 - layout.body_line;
            let (left, width) = (origin.x + GUTTER, layout.content_width());
            kit::paragraph(left, top, width, message, Face::BODY, 3);
            dessen::dsn_refresh();
        };
        let result = account.import(self.clock.offset_minutes, &show);
        let mut model = self.model.borrow_mut();
        match result {
            Ok(()) => {
                model.stats.reload(self.clock.today, self.clock.valid);
                model.offer = false;
            }
            // Cancelled: Confirm opens the book again until the next launch.
            Err(message) => {
                model.offer = !message.is_empty();
                model.note = message;
            }
        }
        model.place_book_card();
        self.stop = 0;
    }
}

impl Screen for ReadStats {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        let page = self.page();
        let focused = (self.focus_ring && page == 1).then(|| (self.stop - 1) as usize);
        let view = PageView {
            model: self.model.clone(),
            page,
            focused,
        };
        let position = kit::join!(page + 1, "/", self.pages());
        NavigationScreen::new(view).title(kit::status(Some(&position)))
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        let offer = self.model.borrow().offer;
        // Page 2 is always there, and ends at its last icon.
        let last = first_stop(self.pages() - 1).max(APP_COUNT);
        match key {
            Button::Confirm if offer && self.page() == 0 => Some(Msg::Import),
            Button::Confirm => Some(Msg::Open(self.stop)),
            // Back on page 1 is left to xpui, which ends the app.
            Button::Back if self.stop == 0 => None,
            Button::Back => Some(Msg::Focus(0)),
            Button::Up | Button::Left => Some(Msg::Focus((self.stop - 1).max(0))),
            Button::Down | Button::Right => Some(Msg::Focus((self.stop + 1).min(last))),
            Button::PageForward => Some(self.turn(true)),
            Button::PageBack => Some(self.turn(false)),
            _ => None,
        }
    }

    fn on_swipe(&self, direction: SwipeDir) -> Option<Msg> {
        match direction {
            SwipeDir::Up | SwipeDir::Left => Some(self.turn(true)),
            SwipeDir::Down | SwipeDir::Right => Some(self.turn(false)),
            SwipeDir::None => None,
        }
    }

    fn on_background_tap(&self, _point: Point) -> Option<Msg> {
        // A tap on empty space turns to the next page, round to the first.
        Some(Msg::Focus(first_stop((self.page() + 1) % self.pages())))
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Focus(stop) => self.stop = stop,
            Msg::Open(stop) => self.open(stop),
            Msg::Import => self.import(),
        }
    }
}
