//! One page, as an xpui view: its cards, and the book card or the app grid.

use alloc::rc::Rc;
use core::cell::RefCell;
use xpui::{InputMask, Interactions, Point, Size, Trigger, View};

use crate::app_grid::APPS;
use crate::layout::offset;
use crate::model::Model;
use crate::screen::Msg;

pub struct PageView {
    pub model: Rc<RefCell<Model>>,
    pub page: i32,
    /// The app icon ringed by the key focus, on page 2.
    pub focused: Option<usize>,
}

impl View<Msg> for PageView {
    fn measure(&mut self, _available: Size) {}

    fn size(&self) -> Size {
        let layout = self.model.borrow().layout;
        Size::new(layout.width, layout.height)
    }

    fn render(&self, origin: Point) {
        self.model.borrow().render(origin, self.page, self.focused);
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<Msg>) {
        let model = self.model.borrow();
        let mut tap = |rect, message| out.declare(rect, InputMask::TAP, Trigger::Message(message));
        if self.page == 0 {
            if model.book_card.book.is_some() {
                tap(offset(model.book_card.rect, origin), Msg::Open(0));
            }
            if model.offer {
                tap(offset(model.layout.import_button(), origin), Msg::Import);
            }
        } else {
            for index in 0..APPS.len() {
                tap(
                    model.layout.app_cell(origin, index),
                    Msg::Open(index as i32 + 1),
                );
            }
        }
    }
}
