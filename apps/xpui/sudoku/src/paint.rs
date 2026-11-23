//! Drawing the board's cells and the number pad's keys.

use xpui::{Point, Rect, Renderer};

use crate::board::{column, row};

/// Box borders' thickness; cell borders are one pixel.
const BOX_BORDER: i32 = 4;

/// A digit centred in `cell`, bold for a printed clue and thin for an entry.
fn digit(cell: Rect, digit: u8, bold: bool) {
    let width = cell.size.width * 3 / 10;
    let stroke = if bold { width / 3 } else { width / 7 }.max(2);
    let origin = Point::new(
        cell.origin.x + (cell.size.width - width) / 2,
        cell.origin.y + (cell.size.height - 2 * width) / 2,
    );
    kit::digit(origin, width, stroke, u32::from(digit));
}

/// Board cell `index`: its borders, a ring when `selected`, and its digit
/// (0 for none).
pub fn cell(index: usize, rect: Rect, value: u8, given: bool, selected: bool) {
    let (left, top, side) = (rect.origin.x, rect.origin.y, rect.size.width);
    if selected {
        Renderer::fill_rect(rect, true);
        Renderer::fill_rect(Rect::new(left + 5, top + 5, side - 10, side - 10), false);
    }
    let border = |starts_box: bool| if starts_box { BOX_BORDER } else { 1 };
    let left_border = border(column(index).is_multiple_of(3));
    let top_border = border(row(index).is_multiple_of(3));
    Renderer::fill_rect(Rect::new(left, top, left_border, side), true);
    Renderer::fill_rect(Rect::new(left, top, side, top_border), true);
    // The last row and column also close the board on their far side.
    if column(index) == 8 {
        Renderer::fill_rect(
            Rect::new(left + side - BOX_BORDER, top, BOX_BORDER, side),
            true,
        );
    }
    if row(index) == 8 {
        Renderer::fill_rect(
            Rect::new(left, top + side - BOX_BORDER, side, BOX_BORDER),
            true,
        );
    }
    if value != 0 {
        digit(rect, value, given);
    }
}

/// Number pad key `index`: 1 to 9, then an X that erases.
pub fn key(index: usize, rect: Rect) {
    let (left, top, side) = (rect.origin.x, rect.origin.y, rect.size.width);
    Renderer::fill_rect(Rect::new(left + 2, top, side - 4, side), true);
    Renderer::fill_rect(Rect::new(left + 4, top + 2, side - 8, side - 4), false);
    if index < 9 {
        digit(rect, index as u8 + 1, false);
    } else {
        let (near, far) = (side / 4, side * 3 / 4);
        let at = |across: i32, down: i32| Point::new(left + across, top + down);
        kit::thick_line(at(near, near), at(far, far), 3);
        kit::thick_line(at(far, near), at(near, far), 3);
    }
}
