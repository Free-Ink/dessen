//! The screen: the board, the level and moves, and how input plays it.

use xpui::{
    Button, CellGrid, Modifiers, NavigationScreen, Point, Rect, Renderer, Screen, Text, View, vstack,
};

use crate::board::{Board, SIDE, SQUARES};

/// Space between squares.
const GAP: i32 = 8;

#[derive(Clone, Copy)]
pub enum Msg {
    Press(usize),
    Cursor(usize),
    NextLevel,
}

pub struct LightsOut {
    board: Board,
    cursor: usize,
    moves: u32,
    level: u32,
}

impl LightsOut {
    pub fn new() -> Self {
        LightsOut {
            board: Board::level(1),
            cursor: 0,
            moves: 0,
            level: 1,
        }
    }

    fn next_level(&mut self) {
        self.level += 1;
        self.board = Board::level(self.level);
        self.moves = 0;
    }
}

/// One square: black when lit, white when not, with a dot on the cursor.
fn paint_square(cell: Rect, lit: bool, cursor: bool) {
    let side = cell.size.width - GAP;
    let (left, top) = (cell.origin.x + GAP / 2, cell.origin.y + GAP / 2);
    Renderer::fill_rect(Rect::new(left, top, side, side), true);
    if !lit {
        Renderer::fill_rect(Rect::new(left + 3, top + 3, side - 6, side - 6), false);
    }
    if cursor {
        let inset = side * 3 / 8;
        let dot = Rect::new(
            left + inset,
            top + inset,
            side - 2 * inset,
            side - 2 * inset,
        );
        Renderer::fill_rect(dot, !lit);
    }
}

impl Screen for LightsOut {
    type Message = Msg;

    fn title(&self) -> Option<&'static str> {
        Some("Lights Out")
    }

    fn body(&self) -> impl View<Msg> {
        let lit: [bool; SQUARES] = core::array::from_fn(|index| self.board.is_lit(index));
        let cursor = self.cursor;
        let board = CellGrid::new(SIDE, SIDE, move |index, cell| {
            paint_square(cell, lit[index], index == cursor);
        })
        .on_cell_tap(Msg::Press);
        let status = if self.board.solved() {
            "Solved! Tap or Confirm for the next level."
        } else {
            "Turn every square white."
        };
        NavigationScreen::new(vstack![12;
            Text::new(kit::join!("Level ", self.level, "   Moves ", self.moves)),
            board.flexible(),
            Text::new(status),
        ])
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        match key {
            Button::Confirm if self.board.solved() => Some(Msg::NextLevel),
            Button::Confirm => Some(Msg::Press(self.cursor)),
            key => kit::step(self.cursor, SQUARES, SIDE, key).map(Msg::Cursor),
        }
    }

    fn on_background_tap(&self, _point: Point) -> Option<Msg> {
        self.board.solved().then_some(Msg::NextLevel)
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Cursor(index) => self.cursor = index,
            // Once solved, any press or tap moves on.
            _ if self.board.solved() => self.next_level(),
            Msg::Press(index) => {
                self.cursor = index;
                self.board.press(index);
                self.moves += 1;
            }
            Msg::NextLevel => {}
        }
    }
}
