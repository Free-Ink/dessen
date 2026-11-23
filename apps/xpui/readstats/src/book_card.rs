//! The card for the book to continue, pinned to the bottom of page 1.

use kit::shelf::Book;
use kit::{Align, Bitmap, Face};
use xpui::host::Theme;
use xpui::{Point, Rect, Renderer};

use crate::layout::{Layout, PAD, offset};

/// The most a cover may take, in bytes.
const COVER_BYTES: usize = 8192;

/// The first book on the recents shelf, with its cover.
pub struct BookCard {
    pub book: Option<Book>,
    cover: Option<Bitmap>,
    /// Where the card is, from the content's top left.
    pub rect: Rect,
}

impl BookCard {
    pub fn new() -> BookCard {
        BookCard {
            book: kit::shelf::page(0, 1).and_then(|mut books| books.pop()),
            cover: None,
            rect: Rect::new(0, 0, 0, 0),
        }
    }

    /// Moves the card to `rect`, loading the cover again when its height
    /// changes: the cover is as tall as the card, and slow the first time
    /// (decoded, then cached).
    pub fn place(&mut self, rect: Rect) {
        let Some(book) = &self.book else { return };
        if rect.size.height != self.rect.size.height {
            // The old cover goes first, so the two are never on the heap at once.
            self.cover = None;
            self.cover = Bitmap::cover(&book.path, rect.size.height - 2 * PAD, COVER_BYTES);
        }
        self.rect = rect;
    }

    /// The card, for the page drawn from `origin`.
    pub fn draw(&self, layout: &Layout, origin: Point) {
        let Some(book) = &self.book else { return };
        let card = offset(self.rect, origin);
        let (left, top) = (card.origin.x, card.origin.y);
        let (width, height) = (card.size.width, card.size.height);
        kit::frame(card, 2);
        let mut text_left = left + PAD;
        if let Some(cover) = &self.cover {
            let cover_top = top + (height - cover.height) / 2;
            cover.draw(Point::new(text_left, cover_top));
            Renderer::stroke_rect(Rect::new(text_left, cover_top, cover.width, cover.height));
            text_left += cover.width + 16;
        }
        let text_width = left + width - PAD - text_left;
        let (small_line, body_line) = (layout.small_line, layout.body_line);
        let text_top = top + (height - (small_line + body_line + small_line + 18)) / 2;
        let small = Face::SMALL;
        kit::text(
            text_left,
            text_top,
            text_width,
            "Continue reading",
            small,
            Align::Left,
        );
        let title_top = text_top + small_line;
        kit::text(
            text_left,
            title_top,
            text_width,
            &book.title,
            Face::BODY.bold(),
            Align::Left,
        );
        let author_top = title_top + body_line;
        kit::text(
            text_left,
            author_top,
            text_width,
            &book.author,
            small,
            Align::Left,
        );
        if let Some(percent) = book.percent {
            let label_width = small.width("100%") + 10;
            let bar_top = author_top + small_line + 8;
            let bar_width = text_width - label_width;
            Theme::draw_progress_bar(Rect::new(text_left, bar_top, bar_width, 10), percent, 100);
            let label = kit::join!(percent, "%");
            let label_top = bar_top + 5 - small_line / 2;
            kit::text(
                text_left + bar_width,
                label_top,
                label_width,
                &label,
                small,
                Align::Right,
            );
        }
    }
}
