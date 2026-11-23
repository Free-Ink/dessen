//! The shelf, a page at a time, and which book is selected.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use kit::Bitmap;
use kit::shelf::{self, Book};

use crate::layout::{COVER_BYTES, COVER_HEIGHT};

/// One page of the shelf, shared with the views that draw it.
pub struct Page {
    pub books: Vec<Book>,
    /// Each book's cover, in the same order; `None` where it has none.
    pub covers: Vec<Option<Bitmap>>,
}

pub struct Recents {
    /// The shelf index of the page's first book.
    pub start: u32,
    pub per_page: usize,
    pub page: Rc<Page>,
    pub selected: usize,
}

impl Recents {
    /// The first page of the shelf, `per_page` books at a time.
    pub fn new(per_page: usize) -> Self {
        let empty = Page {
            books: Vec::new(),
            covers: Vec::new(),
        };
        let mut recents = Recents {
            start: 0,
            per_page,
            page: Rc::new(empty),
            selected: 0,
        };
        recents.show(0);
        recents
    }

    pub fn is_empty(&self) -> bool {
        self.page.books.is_empty()
    }

    /// Shows the page from `start`; false, changing nothing, when it is empty.
    pub fn show(&mut self, start: u32) -> bool {
        let Some(books) = shelf::page(start, self.per_page) else {
            return false;
        };
        // Covers are decoded by the firmware, slowly the first time, and
        // cached after: each page fetches its own before it is drawn.
        let covers = books
            .iter()
            .map(|book| Bitmap::cover(&book.path, COVER_HEIGHT, COVER_BYTES))
            .collect();
        self.page = Rc::new(Page { books, covers });
        self.start = start;
        self.selected = self.selected.min(self.page.books.len() - 1);
        true
    }

    /// Turns to the next page, or the one before: selecting its first book
    /// going forward and its last going back.
    pub fn turn(&mut self, forward: bool) {
        let per_page = self.per_page as u32;
        if forward {
            let full = self.page.books.len() == self.per_page;
            if full && self.show(self.start + per_page) {
                self.selected = 0;
            }
        } else if self.start > 0 && self.show(self.start.saturating_sub(per_page)) {
            self.selected = self.page.books.len() - 1;
        }
    }

    /// Opens book `index` of the page. Ends the app when it succeeds; the
    /// firmware runs this launcher again when the book is closed.
    pub fn open(&self, index: usize) {
        if let Some(book) = self.page.books.get(index) {
            dessen::dsn_open_book(book.path.as_bytes());
        }
    }

    /// "1-4": which books of the shelf this page shows.
    pub fn range(&self) -> String {
        let first = self.start + 1;
        match self.page.books.len() as u32 {
            0 => String::new(),
            1 => kit::join!(first),
            shown => kit::join!(first, "-", self.start + shown),
        }
    }
}
