//! Sizes the screen is built from.

/// Side margin of the list.
pub const GUTTER: i32 = 28;
/// How tall a cover is drawn; a row is built around it.
pub const COVER_HEIGHT: i32 = 92;
/// One book's row.
pub const ROW_HEIGHT: i32 = COVER_HEIGHT + 28;
/// The bar of home destinations along the bottom.
pub const NAV_HEIGHT: i32 = 92;
/// The most a cover thumbnail may take, in bytes.
pub const COVER_BYTES: usize = 1024;

/// As many rows as fit above the bar in content `height` pixels tall, within
/// what one shelf request can return.
pub fn books_per_page(height: i32) -> usize {
    let room = height - NAV_HEIGHT - 8;
    (room / ROW_HEIGHT).clamp(1, dessen::DSN_SHELF_PAGE_MAX) as usize
}
