//! The page of books: cover, title, author and progress, one row each.

use alloc::rc::Rc;
use kit::shelf::Book;
use kit::{Align, Bitmap, Face};
use xpui::host::Theme;
use xpui::{InputMask, Interactions, Point, Rect, Renderer, Size, Trigger, View};

use crate::layout::{COVER_HEIGHT, GUTTER, ROW_HEIGHT};
use crate::recents::Page;
use crate::screen::Msg;

/// The page of books, or what to do while there are none. A tap on a book
/// opens it.
pub struct ShelfView {
    page: Rc<Page>,
    selected: usize,
    size: Size,
}

impl ShelfView {
    pub fn new(page: Rc<Page>, selected: usize) -> Self {
        ShelfView {
            page,
            selected,
            size: Size::default(),
        }
    }

    /// Row `index`, for the view drawn from `origin`.
    fn row(&self, origin: Point, index: usize) -> Rect {
        Rect::new(
            origin.x + GUTTER - 12,
            origin.y + index as i32 * ROW_HEIGHT,
            self.size.width - 2 * GUTTER + 24,
            ROW_HEIGHT,
        )
    }

    fn empty(&self, origin: Point) {
        let message = "No recent books. Open a book or add books and update the Library.";
        let top = origin.y + self.size.height / 2 - 2 * Face::BODY.line_height();
        let width = self.size.width - 2 * GUTTER;
        kit::paragraph(origin.x + GUTTER, top, width, message, Face::BODY, 4);
    }
}

/// One book in `row`: a gray band when `selected`, the cover, then title,
/// author and a progress bar beside it.
fn paint_book(row: Rect, book: &Book, cover: Option<&Bitmap>, selected: bool) {
    let (small, body) = (Face::SMALL, Face::BODY);
    if selected {
        let band = Rect::new(
            row.origin.x,
            row.origin.y + 6,
            row.size.width,
            row.size.height - 12,
        );
        Renderer::fill_rect_dither(band, true);
    }
    let top = row.origin.y + (ROW_HEIGHT - COVER_HEIGHT) / 2;
    let right = row.origin.x + row.size.width - 12;
    let mut left = row.origin.x + 12;
    if let Some(cover) = cover {
        cover.draw(Point::new(left, top));
        Renderer::stroke_rect(Rect::new(left, top, cover.width, cover.height));
        left += cover.width + 20;
    }
    let width = right - left;
    // Title, author and bar, centred against the cover.
    let block = body.line_height() + 6 + small.line_height() + 18 + 10;
    let title_top = top + ((COVER_HEIGHT - block) / 2).max(0);
    kit::text(
        left,
        title_top,
        width,
        &book.title,
        body.bold(),
        Align::Left,
    );
    let author_top = title_top + body.line_height() + 6;
    kit::text(left, author_top, width, &book.author, small, Align::Left);
    let bar_width = width - 70;
    if let (Some(percent), true) = (book.percent, bar_width > 60) {
        let bar_top = author_top + small.line_height() + 18;
        Theme::draw_progress_bar(Rect::new(left, bar_top, bar_width, 10), percent, 100);
        let label_top = bar_top + 5 - small.line_height() / 2;
        let label = kit::join!(percent, "%");
        kit::text(
            left + bar_width + 12,
            label_top,
            58,
            &label,
            small,
            Align::Left,
        );
    }
}

impl View<Msg> for ShelfView {
    fn measure(&mut self, available: Size) {
        self.size = available;
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        if self.page.books.is_empty() {
            self.empty(origin);
            return;
        }
        for (index, book) in self.page.books.iter().enumerate() {
            let cover = self.page.covers.get(index).and_then(Option::as_ref);
            let row = self.row(origin, index);
            paint_book(row, book, cover, index == self.selected);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<Msg>) {
        for index in 0..self.page.books.len() {
            let row = self.row(origin, index);
            out.declare(row, InputMask::TAP, Trigger::Message(Msg::Open(index)));
        }
    }
}
