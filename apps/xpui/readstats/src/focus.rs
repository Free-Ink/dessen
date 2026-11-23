//! The focus Up and Down step through: stop 0 is page 1 (the book card),
//! stops 1 to `APP_COUNT` the icons on page 2, and one stop per page after.

use crate::app_grid::APP_COUNT;

/// The page focus `stop` is on.
pub fn page_of(stop: i32) -> i32 {
    match stop {
        0 => 0,
        stop if stop <= APP_COUNT => 1,
        stop => stop - APP_COUNT + 1,
    }
}

/// The first stop on `page`.
pub fn first_stop(page: i32) -> i32 {
    match page {
        0 => 0,
        1 => 1,
        page => page + APP_COUNT - 1,
    }
}
