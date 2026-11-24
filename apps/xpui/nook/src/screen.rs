//! The screen: what the game holds, and what each tap and button does to it.

use alloc::boxed::Box;

use xpui::{Modal, Scrim, Screen, View};

use crate::board::{self, Board, MAX_SIDE};
use crate::view::{Bare, Help, Table};

#[derive(Clone, Copy)]
pub enum Msg {
    /// A tap on a cubby.
    Tap(usize),
    NextShelf,
    /// The New button: asks first when there is work to lose.
    AskNew,
    /// The answer to that question: 0 is yes.
    Answer(usize),
    KeepPlaying,
    Help,
}

pub struct Nook {
    board: Board,
    shelf: u32,
    /// Whether the player is being asked about a new shelf.
    asking: bool,
}

/// A number in `0..count`, from the device's random source.
fn below(count: usize) -> usize {
    ((u64::from(dessen::dsn_random()) * count as u64) >> 32) as usize
}

/// Shelf `number`, with no plants yet: 6 cubbies square, then 7, then 8 from there on.
fn puzzle(number: u32) -> Board {
    let side = (5 + number as usize).min(MAX_SIDE);
    let mut board = board::shelf(side, &mut below);
    board.plants = 0;
    board
}

impl Nook {
    pub fn new() -> Self {
        Nook {
            board: puzzle(1),
            shelf: 1,
            asking: false,
        }
    }
}

impl Screen for Nook {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        let page: Box<dyn View<Msg>> = Box::new(Table::new(self.board, self.shelf));
        let question = self.asking.then(|| -> Box<dyn View<Msg>> {
            Box::new(
                Modal::confirm("Start a New Shelf?", ["New Shelf", "Keep Playing"])
                    .on_select(Msg::Answer)
                    .on_dismiss(Msg::KeepPlaying)
                    .scrim(Scrim::Dim),
            )
        });
        Bare::new(page, question)
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Tap(cell) => self.board.tap(cell),
            Msg::NextShelf => {
                self.shelf += 1;
                self.board = puzzle(self.shelf);
            }
            // Nothing placed is nothing to lose.
            Msg::AskNew if self.board.plants | self.board.marks == 0 => {
                self.board = puzzle(self.shelf);
            }
            Msg::AskNew => self.asking = true,
            Msg::Answer(choice) => {
                self.asking = false;
                if choice == 0 {
                    self.board = puzzle(self.shelf);
                }
            }
            Msg::KeepPlaying => self.asking = false,
            // A page of its own on the stack; its arrow, or Back, pops it.
            Msg::Help => {
                xpui::present(HowToPlay);
            }
        }
    }
}

/// The page that says how to play, pushed over the game.
struct HowToPlay;

impl Screen for HowToPlay {
    type Message = ();

    fn body(&self) -> impl View<()> {
        Bare::pushed("How to Play", (), Box::new(Help::new()))
    }

    /// Its only message is the arrow back.
    fn update(&mut self, _back: ()) {
        xpui::finish_screen();
    }
}
