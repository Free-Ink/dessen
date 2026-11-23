//! Moving a cursor over a board with the keys.

use xpui::{Button, Input};

/// Moves a cursor over `count` cells laid out `columns` wide, for a key.
///
/// With Left and Right keys, those move along a row and Up and Down by a row.
/// Without them every key is Up or Down, which step one cell, so a board with
/// a single pair of keys still reaches every cell. Both wrap.
pub fn step(cursor: usize, count: usize, columns: usize, key: Button) -> Option<usize> {
    let two_axes = Input::has_left_right_keys();
    // Moving back is moving forward by all but that many, modulo `count`.
    let delta = match key {
        Button::Left => count - 1,
        Button::Right => 1,
        Button::Up if two_axes => count - columns,
        Button::Down if two_axes => columns,
        Button::Up => count - 1,
        Button::Down => 1,
        _ => return None,
    };
    Some((cursor + delta) % count)
}
