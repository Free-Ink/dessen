//! Painting helpers in the firmware's three inks.

use xpui::{Rect, Renderer};

/// Light gray.
pub fn light(left: i32, top: i32, width: i32, height: i32) {
    Renderer::fill_rect_dither(Rect::new(left, top, width, height), true);
}

/// Dark gray.
pub fn dark(left: i32, top: i32, width: i32, height: i32) {
    Renderer::fill_rect_dither(Rect::new(left, top, width, height), false);
}

pub fn black(left: i32, top: i32, width: i32, height: i32) {
    Renderer::fill_rect(Rect::new(left, top, width, height), true);
}

/// One heatmap cell `side` pixels square: level 0 is an outline of light
/// gray, then light, dark and black as the reading grows.
pub fn heat(left: i32, top: i32, side: i32, level: u32) {
    match level {
        0 => {
            light(left, top, side, 1);
            light(left, top + side - 1, side, 1);
            light(left, top, 1, side);
            light(left + side - 1, top, 1, side);
        }
        1 => light(left, top, side, side),
        2 => dark(left, top, side, side),
        _ => black(left, top, side, side),
    }
}

/// A bar's height for `seconds`, of a tallest `max`, in a chart `area` tall.
pub fn bar(seconds: u32, max: u32, area: i32) -> i32 {
    if seconds == 0 {
        return 0;
    }
    let scaled = seconds / 60 * area as u32 / (max / 60).max(1);
    (scaled as i32).clamp(2, area)
}
