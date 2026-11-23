//! The grid of icons for the firmware's own screens, pinned to the bottom of
//! every page after the first.

use kit::{Align, Face};
use xpui::{Point, Rect};

use crate::glyphs::{GLYPH, GLYPHS, TILE, TILE_RADIUS};
use crate::layout::{GAP, GUTTER, Layout, PAD};

/// The grid's entries, in `GLYPHS` order: a label and the `DSN_HOME_*` screen
/// it opens.
pub const APPS: [(&str, i32); 6] = [
    ("Library", dessen::DSN_HOME_LIBRARY),
    ("Files", dessen::DSN_HOME_FILES),
    ("OPDS", dessen::DSN_HOME_OPDS),
    ("Plugins", dessen::DSN_HOME_PLUGINS),
    ("Transfer", dessen::DSN_HOME_TRANSFER),
    ("Settings", dessen::DSN_HOME_SETTINGS),
];
pub const APP_COUNT: i32 = APPS.len() as i32;

/// How the grid's cells are laid out.
pub struct AppGrid {
    pub columns: i32,
    pub cell_width: i32,
    pub cell_height: i32,
    /// From the content's top.
    pub top: i32,
}

impl Layout {
    /// One row on a wide screen, rows of three otherwise.
    pub fn app_grid(&self) -> AppGrid {
        let columns = if self.content_width() >= 640 {
            APP_COUNT
        } else {
            3
        };

        let cell_height = TILE + 6 + self.small_line + GAP;

        AppGrid {
            columns,
            cell_width: self.content_width() / columns,
            cell_height,
            top: self.height - PAD - (APP_COUNT / columns) * cell_height + GAP,
        }
    }

    /// App `index`'s cell, for the page drawn from `origin`: its icon sits at
    /// the top, its label under it.
    pub fn app_cell(&self, origin: Point, index: usize) -> Rect {
        let grid = self.app_grid();
        let (column, row) = (index as i32 % grid.columns, index as i32 / grid.columns);

        Rect::new(
            origin.x + GUTTER + column * grid.cell_width,
            origin.y + grid.top + row * grid.cell_height,
            grid.cell_width,
            grid.cell_height,
        )
    }

    /// The grid, with a ring around app `focused`.
    pub fn draw_apps(&self, origin: Point, focused: Option<usize>) {
        for (index, (label, _)) in APPS.iter().enumerate() {
            let cell = self.app_cell(origin, index);
            let (cell_left, icon_top, width) = (cell.origin.x, cell.origin.y, cell.size.width);
            let icon_left = cell_left + (width - TILE) / 2;
            draw_icon(Point::new(icon_left, icon_top), &GLYPHS[index]);
            let is_focused = focused == Some(index);
            if is_focused {
                let ring = Rect::new(icon_left - 6, icon_top - 6, TILE + 12, TILE + 12);
                kit::frame(ring, 3);
            }
            let face = Face::SMALL.bold_if(is_focused);
            kit::text(
                cell_left,
                icon_top + TILE + 6,
                width,
                label,
                face,
                Align::Center,
            );
        }
    }
}

/// One launcher icon: a black rounded tile, its glyph in white on top. Both
/// are drawn by the firmware from the glyph's own bytes, with nothing copied.
fn draw_icon(origin: Point, glyph: &'static [u8]) {
    let (x, y) = (origin.x, origin.y);
    dessen::dsn_fill_round(
        x,
        y,
        TILE,
        TILE,
        dessen::DSN_BLACK,
        TILE_RADIUS,
        dessen::DSN_CORNERS_ALL,
    );
    let inset = (TILE - GLYPH) / 2;
    dessen::dsn_bitmap(
        x + inset,
        y + inset,
        GLYPH,
        GLYPH,
        glyph,
        GLYPH,
        GLYPH,
        dessen::DSN_BITMAP_BW,
        dessen::DSN_BITMAP_CENTER,
        dessen::DSN_WHITE,
        0,
    );
}
