//! Where everything goes. The sizes come from the content area and the
//! firmware's line heights, so the layout scales with the board's fonts.

use xpui::host::Theme;
use xpui::{Point, Rect};

use kit::Face;

use crate::log::Stats;

/// Side margin.
pub const GUTTER: i32 = 28;
/// Padding inside a card.
pub const PAD: i32 = 12;
/// Space between cards.
pub const GAP: i32 = 12;
/// The tallest the book card's cover is drawn.
const COVER_MAX: i32 = 240;
/// Where a page's first card starts.
const PAGE_TOP: i32 = GAP;
/// Where the empty first page's message starts.
pub const EMPTY_TOP: i32 = PAGE_TOP + GAP;

/// `rect`, laid out from the content's top left, moved to `origin`.
pub fn offset(rect: Rect, origin: Point) -> Rect {
    let moved = Point::new(origin.x + rect.origin.x, origin.y + rect.origin.y);
    Rect::new(moved.x, moved.y, rect.size.width, rect.size.height)
}

/// The cards laid over pages, as `Layout::flow` finds them.
pub struct Flow {
    /// Pages in all; there are always at least two, for the app grid.
    pub pages: i32,
    /// Just below the last card on the page asked about.
    pub end: i32,
}

/// The content area and the firmware's line heights.
#[derive(Clone, Copy)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub small_line: i32,
    pub body_line: i32,
    pub title_line: i32,
}

impl Layout {
    pub fn new() -> Layout {
        let area = Theme::content_area();
        Layout {
            width: area.size.width,
            height: area.size.height,
            small_line: Face::SMALL.line_height(),
            body_line: Face::BODY.line_height(),
            title_line: Face::TITLE.line_height(),
        }
    }

    /// The width inside the gutters.
    pub fn content_width(&self) -> i32 {
        self.width - 2 * GUTTER
    }

    /// Height of a card's label row; content starts this far below the
    /// card's top.
    pub fn head(&self) -> i32 {
        PAD / 2 + self.small_line + 6
    }

    /// The smallest book card: room for its text or a 92 pixel cover.
    pub fn book_card_min(&self) -> i32 {
        let text = self.small_line + self.body_line + self.small_line + 18;
        2 * PAD + text.max(92)
    }

    /// The book card, pinned to the bottom of page 1 and grown into whatever
    /// the content above it (ending at `end`) leaves free, so the cover can
    /// be large.
    pub fn book_card(&self, end: i32) -> Rect {
        let bottom = self.height - PAD;
        let height = (bottom - end).clamp(self.book_card_min(), 2 * PAD + COVER_MAX);
        Rect::new(GUTTER, bottom - height, self.content_width(), height)
    }

    /// Where page 1's own content ends, for sizing the book card under it.
    pub fn first_page_end(&self, stats: &Stats, has_book: bool, offer: bool) -> i32 {
        if stats.sessions > 0 {
            self.flow(stats, has_book, Point::new(0, 0), 0, false).end
        } else if offer {
            // The note's lines stay reserved, so an error doesn't move the card.
            let button = self.import_button();
            button.origin.y + button.size.height + GAP + 3 * self.small_line + GAP
        } else {
            EMPTY_TOP + 3 * self.body_line + GAP
        }
    }

    /// The "Import from CrossPoint Sync" button on the empty first page.
    pub fn import_button(&self) -> Rect {
        let top = EMPTY_TOP + 3 * self.body_line + 2 * GAP;
        Rect::new(
            GUTTER,
            top,
            self.content_width(),
            self.body_line + 2 * PAD + 8,
        )
    }

    /// Lays the cards out over pages, leaving room for the book card on page
    /// 1 and the app grid on the others, and draws those on `page` when
    /// `draw`.
    pub fn flow(
        &self,
        stats: &Stats,
        has_book: bool,
        origin: Point,
        page: i32,
        draw: bool,
    ) -> Flow {
        let bottom = self.height - PAD;
        let footer = |page: i32| {
            if page == 0 && has_book {
                self.book_card_min() + GAP
            } else {
                bottom - self.app_grid().top + GAP
            }
        };
        let (mut current, mut top, mut end) = (0, PAGE_TOP, PAGE_TOP);
        for card in crate::cards::Card::ALL {
            let height = if stats.sessions > 0 {
                self.card(card, stats, origin, 0, false)
            } else {
                0
            };
            if height == 0 {
                continue;
            }
            if top > PAGE_TOP && top + height > bottom - footer(current) {
                current += 1;
                top = PAGE_TOP;
            }
            if current == page {
                if draw {
                    self.card(card, stats, origin, origin.y + top, true);
                }
                end = top + height + GAP;
            }
            top += height + GAP;
        }
        Flow {
            pages: (current + 1).max(2),
            end,
        }
    }
}
