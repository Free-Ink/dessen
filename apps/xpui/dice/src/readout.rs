//! The roll in big seven-segment digits.

use xpui::{InputMask, Interactions, Point, Rect, Size, Trigger, View};

use crate::screen::Msg;

/// The roll, as wide as fits; a tap rolls again.
pub struct Readout {
    value: u32,
    size: Size,
}

impl Readout {
    pub fn new(value: u32) -> Self {
        Readout {
            value,
            size: Size::default(),
        }
    }
}

impl View<Msg> for Readout {
    fn measure(&mut self, available: Size) {
        let width = (available.width / 6).min(available.height / 2).max(15);
        self.size = Size::new(available.width, 2 * width);
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        let centre = origin.x + self.size.width / 2;
        kit::number(centre, origin.y, self.size.height / 2, self.value);
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<Msg>) {
        let rect = Rect {
            origin,
            size: self.size,
        };
        out.declare(rect, InputMask::TAP, Trigger::Message(Msg::Roll));
    }
}
