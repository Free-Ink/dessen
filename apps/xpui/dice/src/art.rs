//! How each die is drawn: its outline, inner facets and name.

use xpui::{Font, FontStyle, Point, Rect, Renderer};

use crate::dice::NAMES;

/// A point on the 16 × 16 grid the dice are drawn on.
type GridPoint = (i32, i32);

/// Each die's outline on the grid, as a closed polygon.
const OUTLINES: [&[GridPoint]; 7] = [
    &[(8, 1), (15, 14), (1, 14)],
    &[(2, 2), (14, 2), (14, 14), (2, 14)],
    &[(8, 1), (15, 8), (8, 15), (1, 8)],
    &[(8, 1), (15, 6), (8, 15), (1, 6)],
    &[(8, 1), (15, 6), (12, 15), (4, 15), (1, 6)],
    &[(8, 1), (14, 4), (14, 12), (8, 15), (2, 12), (2, 4)],
    &[
        (5, 1),
        (11, 1),
        (15, 5),
        (15, 11),
        (11, 15),
        (5, 15),
        (1, 11),
        (1, 5),
    ],
];

/// A line on the grid: the die it belongs to, then from and to.
type Facet = (usize, GridPoint, GridPoint);

/// Inner facet lines.
const FACETS: [Facet; 7] = [
    (2, (1, 8), (15, 8)),
    (3, (1, 6), (8, 10)),
    (3, (8, 10), (15, 6)),
    (3, (8, 10), (8, 15)),
    (5, (8, 4), (13, 12)),
    (5, (13, 12), (3, 12)),
    (5, (3, 12), (8, 4)),
];

/// Paints die `index` in `cell`, its outline heavier when `selected`, with
/// its name below.
pub fn paint_die(index: usize, cell: Rect, selected: bool) {
    let Some(outline) = OUTLINES.get(index) else {
        return;
    };
    let font = Font::ui_small();
    let label_height = font.line_height();
    let side = (cell.size.width - 16)
        .min(cell.size.height - label_height - 8)
        .max(16);
    let left = cell.origin.x + (cell.size.width - side) / 2;
    let top = cell.origin.y + 4;
    let at = |(grid_x, grid_y): GridPoint| {
        Point::new(left + grid_x * side / 16, top + grid_y * side / 16)
    };
    let width = if selected { 5 } else { 2 };
    for (corner, next) in outline.iter().zip(outline.iter().cycle().skip(1)) {
        kit::thick_line(at(*corner), at(*next), width);
    }
    for (die, from, to) in FACETS {
        if die == index {
            Renderer::draw_line(at(from), at(to));
        }
    }
    let name = NAMES[index];
    let name_left = cell.origin.x + (cell.size.width - font.text_width(name)) / 2;
    Renderer::draw_text(
        Point::new(name_left, top + side + 4),
        name,
        font.id(),
        FontStyle::Regular,
    );
}
