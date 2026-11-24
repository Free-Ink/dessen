//! The game's two pages, each one view that lays itself out: the table (the
//! bar of buttons, the bookcase with its numbers, the line under it) and the
//! page that says how to play. Both follow the screen standards in
//! docs/writing-apps.md: a bar across the top, the content, a line under it.

use alloc::boxed::Box;
use alloc::string::String;

use kit::Face;
use xpui::{InputMask, Interactions, Point, Rect, Renderer, Size, Theme, Trigger, View};

use crate::board::{Board, Content};
use crate::paint::{self, Case, Count};
use crate::screen::Msg;

/// Space between the bar, the bookcase and the line under it.
const GAP: i32 = 14;
/// Space between a number's plaque and the case.
const PLAQUE_GAP: i32 = 6;

const BODY: i32 = dessen::DSN_FONT_BODY;
const BOLD: i32 = dessen::DSN_TEXT_BOLD;

/// Where the table's parts sit, from its own top-left corner.
struct Layout {
    /// The height of the bar, and of the line under the bookcase.
    bar: i32,
    new: Rect,
    help: Rect,
    case: Case,
    /// The cubbies, each `cell` square.
    grid: Rect,
    cell: i32,
    plaque: Size,
    footer: Rect,
    next: Rect,
}

/// The height of the bar across the top of a page: a line of text with room
/// round it, which is also a comfortable thing to tap.
fn bar_height() -> i32 {
    Face::BODY.line_height() + 16
}

fn layout(size: Size, side: usize) -> Layout {
    let across = side as i32;
    let line = Face::BODY.line_height();
    let bar = bar_height();
    let bold = Face::BODY.bold();
    let help = Rect::new(size.width - bar, 0, bar, bar);
    let new_wide = bold.width("New") + 36;
    let new = Rect::new(help.x() - 10 - new_wide, 0, new_wide, bar);
    let footer = Rect::new(0, size.height - bar, size.width, bar);
    let next_wide = bold.width("Next Shelf") + 40;
    let next = Rect::new(size.width - next_wide, footer.y(), next_wide, bar);

    let case = Case::new(size.width);
    let plaque = Size::new(line + 14, line + 4);
    // What the case and the numbers take beyond the cubbies themselves.
    let over = plaque.height + PLAQUE_GAP + case.crown + case.panel;
    let under = case.plinth + case.plinth / 3;
    let beside = plaque.width + PLAQUE_GAP + case.lip + case.panel;
    let room = Size::new(
        size.width - beside - case.panel - case.lip,
        size.height - 2 * (bar + GAP) - over - under,
    );
    let cell = (room.width / across).min(room.height / across).max(8);
    let wide = cell * across;
    let left = ((size.width - wide) / 2).max(beside);
    let top = bar + GAP + over + (room.height - wide).max(0) / 3;
    Layout {
        bar,
        new,
        help,
        case,
        grid: Rect::new(left, top, wide, wide),
        cell,
        plaque: Size::new(plaque.width.min(cell - 4), plaque.height),
        footer,
        next,
    }
}

/// `rect`, which is measured from a view's corner, moved to where that
/// corner is.
fn placed(rect: Rect, origin: Point) -> Rect {
    Rect::new(origin.x + rect.x(), origin.y + rect.y(), rect.size.width, rect.size.height)
}

fn count(have: u8, want: u8) -> Count {
    match have.cmp(&want) {
        core::cmp::Ordering::Less => Count::Short,
        core::cmp::Ordering::Equal => Count::Met,
        core::cmp::Ordering::Greater => Count::Over,
    }
}

/// The line under the bookcase: the most useful thing to say about the
/// shelf as it stands.
fn hint(board: &Board) -> &'static str {
    let cells = board.side * board.side;
    let touching = (0..cells).any(|cell| board.content(cell) == Content::Plant && board.crowded(cell));
    let too_many = (0..board.side).any(|lane| {
        board.row_plants(lane) > board.rows[lane] || board.column_plants(lane) > board.columns[lane]
    });
    if touching {
        "Plants marked ! are touching."
    } else if too_many {
        "A grey number has too many plants."
    } else if board.plants == 0 {
        "Tap a cubby to place a plant."
    } else if board.plants.count_ones() >= board.books.count_ones() {
        "A book has no plant beside it."
    } else {
        "Tap again to cross out, then clear."
    }
}

/// The game itself.
pub struct Table {
    board: Board,
    shelf: u32,
    size: Size,
}

impl Table {
    pub fn new(board: Board, shelf: u32) -> Table {
        Table {
            board,
            shelf,
            size: Size::new(0, 0),
        }
    }
}

impl View<Msg> for Table {
    fn measure(&mut self, available: Size) {
        self.size = available;
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        let board = &self.board;
        let parts = layout(self.size, board.side);
        let at = |rect: Rect| placed(rect, origin);

        // The bar: where the player is, and the two buttons.
        let title = kit::join!("Shelf ", self.shelf);
        let title_wide = Face::BODY.bold().width(&title);
        paint::label(at(Rect::new(0, 0, title_wide + 4, parts.bar)), &title, BODY, true, BOLD);
        let (have, want) = (board.plants.count_ones(), board.books.count_ones());
        let tally = kit::join!(have, " of ", want, " plants");
        let tally_left = title_wide + 20;
        let tally_box = Rect::new(tally_left, 0, parts.new.x() - tally_left - 8, parts.bar);
        paint::label(at(tally_box), &tally, BODY, true, 0);
        paint::button(at(parts.new), "New", false);
        paint::button(at(parts.help), "?", false);

        // The bookcase, a number over each column and beside each row.
        let (grid, cell, case) = (at(parts.grid), parts.cell, &parts.case);
        case.paint(grid);
        let (plaque_wide, plaque_tall) = (parts.plaque.width, parts.plaque.height);
        let over = grid.y() - case.panel - case.crown - PLAQUE_GAP - plaque_tall;
        let beside = grid.x() - case.panel - case.lip - PLAQUE_GAP - plaque_wide;
        for lane in 0..board.side {
            let along = lane as i32 * cell;
            let column = Rect::new(grid.x() + along + (cell - plaque_wide) / 2, over, plaque_wide, plaque_tall);
            paint::clue(column, board.columns[lane], count(board.column_plants(lane), board.columns[lane]));
            let row = Rect::new(beside, grid.y() + along + (cell - plaque_tall) / 2, plaque_wide, plaque_tall);
            paint::clue(row, board.rows[lane], count(board.row_plants(lane), board.rows[lane]));
        }
        for index in 0..board.side * board.side {
            let (row, column) = ((index / board.side) as i32, (index % board.side) as i32);
            let rect = Rect::new(grid.x() + column * cell, grid.y() + row * cell, cell, cell);
            // The books differ by cubby and by shelf, never by redraw.
            let seed = (board.books as u32 ^ (board.books >> 32) as u32)
                .wrapping_add(index as u32 * 0x9e37_79b9)
                | 1;
            let content = board.content(index);
            let crowded = content == Content::Plant && board.crowded(index);
            paint::cubby(rect, content, seed, crowded);
        }

        // The line under it: what to do, or what comes next.
        let footer = at(parts.footer);
        if board.solved() {
            paint::label(footer, "Shelf Complete!", BODY, true, BOLD);
            paint::button(at(parts.next), "Next Shelf", true);
        } else {
            // Centred on the bookcase, which the numbers push off the middle,
            // in the widest box that centre allows.
            let middle = parts.grid.x() + parts.grid.size.width / 2;
            let half = middle.min(self.size.width - middle);
            let under = Rect::new(middle - half, parts.footer.y(), 2 * half, parts.bar);
            paint::label(at(under), hint(board), BODY, false, 0);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<Msg>) {
        let parts = layout(self.size, self.board.side);
        let tap = InputMask::TAP;
        out.declare(placed(parts.new, origin), tap, Trigger::Message(Msg::AskNew));
        out.declare(placed(parts.help, origin), tap, Trigger::Message(Msg::Help));
        if self.board.solved() {
            out.declare(placed(parts.footer, origin), tap, Trigger::Message(Msg::NextShelf));
            return;
        }
        let (side, cell) = (self.board.side, parts.cell);
        let located = move |at: Point| {
            let column = (at.x / cell).clamp(0, side as i32 - 1) as usize;
            let row = (at.y / cell).clamp(0, side as i32 - 1) as usize;
            Msg::Tap(row * side + column)
        };
        out.declare(placed(parts.grid, origin), tap, Trigger::Located(Box::new(located)));
    }
}

/// The rules, a paragraph each.
const RULES: [&str; 4] = [
    "Every book needs a plant of its own, in the cubby above, below or beside it.",
    "Plants never touch each other, not even at a corner.",
    "The numbers say how many plants each row and column holds. A number turns solid when its line is right.",
    "Tap a cubby for a plant, again to cross it out, and again to clear it.",
];

/// Paints `text` from (`left`, `top`) in lines no wider than `width`, broken
/// at spaces. Returns where the next line would start.
fn flow(left: i32, mut top: i32, width: i32, text: &str, face: Face) -> i32 {
    let mut line = String::new();
    for word in text.split(' ') {
        if line.is_empty() {
            line.push_str(word);
            continue;
        }
        let longer = kit::join!(line, " ", word);
        if face.width(&longer) > width {
            face.draw(Point::new(left, top), &line);
            top += face.line_height();
            line = String::from(word);
        } else {
            line = longer;
        }
    }
    face.draw(Point::new(left, top), &line);
    top + face.line_height()
}

/// How to play: pictures of what is right and what is not, then the rules.
/// The header over it, with the arrow back, is the screen's (see [`Bare`]).
pub struct Help {
    size: Size,
}

impl Help {
    pub fn new() -> Help {
        Help {
            size: Size::new(0, 0),
        }
    }
}

impl<M> View<M> for Help {
    fn measure(&mut self, available: Size) {
        self.size = available;
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        let (wide, tall) = (self.size.width, self.size.height);
        // The smaller face where the page is short.
        let face = if tall >= 20 * Face::BODY.line_height() {
            Face::BODY
        } else {
            Face::SMALL
        };
        let mut top = origin.y + GAP;

        // Two rows of pieces of shelf. Right: a plant beside its book, above
        // it, or below it. Not right: plants touching, side by side or at a
        // corner.
        let cell = (wide / 8).min(Face::BODY.line_height() * 2).max(24);
        let (mark, first, between) = (origin.x + 16, origin.x + 44, 14);
        let cubby = |left: i32, top: i32, content: Content, seed: u32| {
            let crowded = content == Content::Plant && seed % 2 == 0;
            paint::cubby(Rect::new(left, top, cell, cell), content, seed, crowded);
        };
        // Odd seeds are plants in their place, even ones are touching.
        let (empty, book) = (Content::Empty, Content::Book);
        paint::tick(Point::new(mark, top + cell), 10);
        let mut left = first;
        cubby(left, top, empty, 1);
        cubby(left + cell, top, empty, 1);
        cubby(left, top + cell, book, 7);
        cubby(left + cell, top + cell, Content::Plant, 9);
        left += 2 * cell + between;
        cubby(left, top, Content::Plant, 13);
        cubby(left, top + cell, book, 3);
        left += cell + between;
        cubby(left, top, book, 11);
        cubby(left, top + cell, Content::Plant, 21);
        top += 2 * cell + 16;

        paint::cross(Point::new(mark, top + cell), 10);
        left = first;
        cubby(left, top, empty, 1);
        cubby(left + cell, top, empty, 1);
        cubby(left, top + cell, Content::Plant, 6);
        cubby(left + cell, top + cell, Content::Plant, 18);
        left += 2 * cell + between;
        cubby(left, top, Content::Plant, 26);
        cubby(left + cell, top, empty, 1);
        cubby(left, top + cell, empty, 1);
        cubby(left + cell, top + cell, Content::Plant, 30);
        top += 2 * cell + 18;

        for rule in RULES {
            top = flow(origin.x, top, wide, rule, face) + face.line_height() / 2;
        }
    }
}

/// Space above the bar, on every screen.
const TOP: i32 = 8;

/// The whole screen, in the theme's side margins, with a dialog over it when
/// there is one. Every screen's bar starts `TOP` from the top of the panel: a
/// root screen's page draws its own, and a pushed screen gets one here, with
/// the arrow back, its title, and a rule under it.
pub struct Bare<M> {
    page: Box<dyn View<M>>,
    dialog: Option<Box<dyn View<M>>>,
    /// A pushed screen's title, and the message its arrow sends.
    header: Option<(&'static str, M)>,
    size: Size,
}

impl<M: Clone> Bare<M> {
    /// A root screen.
    pub fn new(page: Box<dyn View<M>>, dialog: Option<Box<dyn View<M>>>) -> Bare<M> {
        Bare {
            page,
            dialog,
            header: None,
            size: Size::new(0, 0),
        }
    }

    /// A pushed screen titled `title`, whose arrow sends `back`.
    pub fn pushed(title: &'static str, back: M, page: Box<dyn View<M>>) -> Bare<M> {
        Bare {
            page,
            dialog: None,
            header: Some((title, back)),
            size: Size::new(0, 0),
        }
    }

    /// Where the page goes: between the theme's side margins, from `TOP` (or
    /// from under a pushed screen's bar) down to where its content area ends.
    fn area(&self) -> Rect {
        let content = Theme::content_area();
        let bottom = content.y() + content.size.height;
        let top = match self.header {
            Some(_) => TOP + bar_height() + 1,
            None => TOP,
        };
        Rect::new(content.x(), top, content.size.width, bottom - top)
    }
}

impl<M: Clone> View<M> for Bare<M> {
    fn measure(&mut self, available: Size) {
        let area = self.area().size;
        self.page.measure(area);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.measure(available);
        }
        self.size = available;
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        if let Some((title, _)) = &self.header {
            let (inset, bar) = (Theme::content_area().x(), bar_height());
            paint::back_arrow(Rect::new(inset, TOP, bar, bar));
            let heading = Rect::new(inset + bar, TOP, self.size.width - 2 * inset - bar, bar);
            paint::label(heading, title, BODY, true, BOLD);
            let rule = TOP + bar;
            Renderer::draw_line(Point::new(0, rule), Point::new(self.size.width - 1, rule));
        }
        let area = self.area();
        self.page.render(Point::new(origin.x + area.x(), origin.y + area.y()));
        if let Some(dialog) = &self.dialog {
            dialog.render(origin);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        if let Some((_, back)) = &self.header {
            // The arrow and the title beside it: a generous way back.
            let target = Rect::new(0, 0, self.size.width / 2, TOP + bar_height());
            out.declare(target, InputMask::TAP, Trigger::Message(back.clone()));
        }
        let area = self.area();
        self.page.interactions(Point::new(origin.x + area.x(), origin.y + area.y()), out);
        // After the page: a dialog that captures input discards what came before.
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.interactions(origin, out);
        }
    }
}
