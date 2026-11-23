//! The bar of home destinations along the bottom, one tappable cell each.

use kit::{Align, Bitmap, Face};
use xpui::{InputMask, Interactions, Point, Rect, Renderer, Size, Trigger, View};

use crate::layout::NAV_HEIGHT;
use crate::screen::Msg;

// ICON_SZ and the IC_* icons, from apps/home_icons.h (see build.rs).
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// A firmware screen per cell, icon over label: its label, `DSN_HOME_*`
/// destination and icon.
pub const NAV: [(&str, i32, &[u8]); 5] = [
    ("Files", dessen::DSN_HOME_FILES, &IC_FOLDER),
    ("OPDS", dessen::DSN_HOME_OPDS, &IC_GLOBE),
    ("Plugins", dessen::DSN_HOME_PLUGINS, &IC_PUZZLE),
    ("Transfer", dessen::DSN_HOME_TRANSFER, &IC_TRANSFER),
    ("Settings", dessen::DSN_HOME_SETTINGS, &IC_SETTINGS),
];

/// The bar; a tap on a cell sends `Msg::Nav` with its index.
pub struct NavBar {
    width: i32,
}

impl NavBar {
    pub fn new() -> Self {
        NavBar { width: 0 }
    }

    /// Cell `index`, for the bar drawn from `origin`.
    fn cell(&self, origin: Point, index: usize) -> Rect {
        let edge = |cell: usize| origin.x + cell as i32 * self.width / NAV.len() as i32;
        Rect::new(
            edge(index),
            origin.y,
            edge(index + 1) - edge(index),
            NAV_HEIGHT,
        )
    }
}

impl View<Msg> for NavBar {
    fn measure(&mut self, available: Size) {
        self.width = available.width;
    }

    fn size(&self) -> Size {
        Size::new(self.width, NAV_HEIGHT)
    }

    fn render(&self, origin: Point) {
        Renderer::fill_rect_dither(Rect::new(origin.x, origin.y, self.width, 1), true);
        for (index, (label, _, icon)) in NAV.iter().enumerate() {
            let cell = self.cell(origin, index);
            let centre = cell.origin.x + cell.size.width / 2;
            if let Some(icon) = Bitmap::new(ICON_SZ, ICON_SZ, icon) {
                let icon_top = origin.y + 28 - ICON_SZ / 2;
                icon.draw(Point::new(centre - ICON_SZ / 2, icon_top));
            }
            let label_top = origin.y + 48;
            let (left, width) = (cell.origin.x, cell.size.width);
            kit::text(left, label_top, width, label, Face::SMALL, Align::Center);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<Msg>) {
        for index in 0..NAV.len() {
            let cell = self.cell(origin, index);
            out.declare(cell, InputMask::TAP, Trigger::Message(Msg::Nav(index)));
        }
    }
}
