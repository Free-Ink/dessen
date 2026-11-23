//! Lines and outlines thicker than xpui's one pixel.

use xpui::{Point, Rect, Renderer};

/// A line `width` pixels thick, drawn as that many one-pixel lines side by
/// side.
pub fn thick_line(from: Point, to: Point, width: i32) {
    let steep = (to.y - from.y).abs() > (to.x - from.x).abs();
    for step in 0..width.max(1) {
        let offset = step - width / 2;
        // Spread across the line: sideways for a steep one, else vertically.
        let (shift_x, shift_y) = if steep { (offset, 0) } else { (0, offset) };
        Renderer::draw_line(
            Point::new(from.x + shift_x, from.y + shift_y),
            Point::new(to.x + shift_x, to.y + shift_y),
        );
    }
}

/// An outline `thickness` pixels wide, inside `rect`.
pub fn frame(rect: Rect, thickness: i32) {
    for inset in 0..thickness.max(1) {
        Renderer::stroke_rect(Rect::new(
            rect.origin.x + inset,
            rect.origin.y + inset,
            rect.size.width - 2 * inset,
            rect.size.height - 2 * inset,
        ));
    }
}
