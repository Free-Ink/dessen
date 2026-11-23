//! The screen: the board, the number pad, and how input fills cells.

use alloc::string::String;
use xpui::{Button, CellGrid, Modifiers, NavigationScreen, Point, Screen, Text, View, vstack};

use crate::board::{Board, CELLS};
use crate::{generate, paint};

/// The cell selected first: the centre.
const CENTRE: usize = 40;
/// Keys on the number pad: 1 to 9 and erase.
const PAD_KEYS: usize = 10;

#[derive(Clone, Copy)]
pub enum Msg {
    Select(usize),
    /// A number pad key: 0 to 8 for 1 to 9, 9 to erase.
    Enter(usize),
    /// Confirm on a cell: its next digit.
    Step,
    NewPuzzle,
}

pub struct Sudoku {
    board: Board,
    selected: usize,
}

impl Sudoku {
    pub fn new(board: Board) -> Self {
        Sudoku {
            board,
            selected: CENTRE,
        }
    }

    fn status(&self) -> String {
        let left = self.board.empty();
        if self.board.solved() {
            String::from("Solved! Tap or Confirm for a new puzzle.")
        } else if left == 0 {
            String::from("Full, but something is wrong.")
        } else {
            kit::join!(left, " cells left. Tap a cell, then a number.")
        }
    }
}

impl Screen for Sudoku {
    type Message = Msg;

    fn title(&self) -> Option<&'static str> {
        Some("Sudoku")
    }

    fn body(&self) -> impl View<Msg> {
        let (grid, given, selected) = (self.board.grid, self.board.given, self.selected);
        let board = CellGrid::new(9, 9, move |index, rect| {
            paint::cell(index, rect, grid[index], given[index], index == selected);
        })
        .on_cell_tap(Msg::Select);
        let pad = CellGrid::new(PAD_KEYS, 1, paint::key).on_cell_tap(Msg::Enter);
        NavigationScreen::new(vstack![12; board.flexible(), pad, Text::new(self.status())])
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        match key {
            Button::Confirm if self.board.solved() => Some(Msg::NewPuzzle),
            Button::Confirm => Some(Msg::Step),
            key => kit::step(self.selected, CELLS, 9, key).map(Msg::Select),
        }
    }

    fn on_background_tap(&self, _point: Point) -> Option<Msg> {
        self.board.solved().then_some(Msg::NewPuzzle)
    }

    fn update(&mut self, message: Msg) {
        if self.board.solved() {
            if matches!(message, Msg::NewPuzzle | Msg::Select(_) | Msg::Enter(_)) {
                self.board = generate::puzzle();
            }
            return;
        }
        let cell = self.selected;
        match message {
            Msg::Select(index) => self.selected = index,
            Msg::Enter(key) if !self.board.given[cell] => {
                let digit = if key < 9 { key as u8 + 1 } else { 0 };
                self.board.set(cell, digit);
            }
            Msg::Step if !self.board.given[cell] => {
                let next = (self.board.grid[cell] + 1) % 10;
                self.board.set(cell, next);
            }
            _ => {}
        }
    }
}
