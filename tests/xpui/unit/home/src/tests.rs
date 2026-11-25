use crate::layout::{NAV_HEIGHT, ROW_HEIGHT, books_per_page};

#[test]
fn books_per_page_fills_the_room_above_the_bar() {
    assert_eq!(books_per_page(480 - 40), 2, "800x480, under the title");
    assert_eq!(books_per_page(800 - 40), 5);
    assert_eq!(books_per_page(NAV_HEIGHT + 8 + ROW_HEIGHT), 1);
    assert_eq!(books_per_page(NAV_HEIGHT + 8 + 3 * ROW_HEIGHT - 1), 2);
}

#[test]
fn books_per_page_is_at_least_one_and_at_most_a_shelf_request() {
    assert_eq!(books_per_page(0), 1);
    assert_eq!(books_per_page(-500), 1);
    assert_eq!(books_per_page(10_000), dessen::DSN_SHELF_PAGE_MAX as usize);
}
