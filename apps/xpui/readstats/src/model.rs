//! What the pages show: the stats, the book card, and the import offer.

use kit::{Align, Face};
use xpui::Point;

use crate::book_card::BookCard;
use crate::layout::{EMPTY_TOP, GAP, GUTTER, Layout, offset};
use crate::log::Stats;

/// Shared by the screen and its page views, and changed only by the screen.
pub struct Model {
    pub layout: Layout,
    pub stats: &'static mut Stats,
    pub book_card: BookCard,
    /// Whether page 1 offers to import from CrossPoint Sync.
    pub offer: bool,
    /// What the last import attempt has to say, under the button.
    pub note: &'static str,
}

impl Model {
    pub fn new(stats: &'static mut Stats, offer: bool) -> Model {
        let mut model = Model {
            layout: Layout::new(),
            stats,
            book_card: BookCard::new(),
            offer,
            note: "",
        };
        model.place_book_card();
        model
    }

    pub fn pages(&self) -> i32 {
        let has_book = self.book_card.book.is_some();
        let origin = Point::new(0, 0);
        self.layout
            .flow(self.stats, has_book, origin, 0, false)
            .pages
    }

    /// Sizes the book card to the room page 1 leaves it: at launch, and when
    /// an import fills or dismisses the empty page.
    pub fn place_book_card(&mut self) {
        let has_book = self.book_card.book.is_some();
        let end = self.layout.first_page_end(self.stats, has_book, self.offer);
        self.book_card.place(self.layout.book_card(end));
    }

    /// Draws `page` from `origin`, with app icon `focused` ringed.
    pub fn render(&self, origin: Point, page: i32, focused: Option<usize>) {
        let has_book = self.book_card.book.is_some();
        self.layout.flow(self.stats, has_book, origin, page, true);
        if page == 0 {
            if self.stats.sessions == 0 {
                self.render_empty(origin);
            }
            self.book_card.draw(&self.layout, origin);
        } else {
            self.layout.draw_apps(origin, focused);
        }
    }

    /// Page 1 with no sessions: what to do, and the import button when
    /// there is an account to import from.
    fn render_empty(&self, origin: Point) {
        let message = if self.stats.error == dessen::DSN_ERR_UNAVAILABLE {
            "Reading stats need newer firmware."
        } else if self.offer {
            "No reading sessions yet. Read a book, or bring your history from CrossPoint Sync."
        } else {
            "No reading sessions yet. Read a book and come back."
        };
        let (left, width) = (origin.x + GUTTER, self.layout.content_width());
        kit::paragraph(left, origin.y + EMPTY_TOP, width, message, Face::BODY, 3);
        if !self.offer {
            return;
        }
        let button = offset(self.layout.import_button(), origin);
        kit::frame(button, 3);
        let (button_top, button_height) = (button.origin.y, button.size.height);
        let label_top = button_top + (button_height - self.layout.body_line) / 2;
        let label = "Import from CrossPoint Sync";
        let (button_left, button_width) = (button.origin.x, button.size.width);
        kit::text(
            button_left,
            label_top,
            button_width,
            label,
            Face::BODY.bold(),
            Align::Center,
        );
        let note_top = button_top + button_height + GAP;
        kit::paragraph(left, note_top, width, self.note, Face::SMALL, 3);
    }
}
