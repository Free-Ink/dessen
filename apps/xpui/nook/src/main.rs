//! Book Nook: a bookcase of cubbies, some holding books. Give every book a
//! plant in a cubby beside it, with no two plants touching; the numbers say
//! how many plants each row and column holds. Every shelf is generated on the
//! device and has exactly one answer. Touch only: tap a cubby for a plant,
//! again to cross it out, again to clear it; New deals another shelf and ?
//! says how to play.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod board;
mod paint;
mod screen;
mod view;

use xpui::App;

// Measured on the VM with an 8 x 8 shelf, the largest, through the help page
// (a second screen on the stack) and the new-shelf question: heap 1064 B at
// most, stack 2372 B at most (see docs/writing-apps.md); these leave room to
// spare.
dessen::heap!(1792);
dessen::stack!(3584);

fn app() -> App {
    App::new(screen::Nook::new())
}

xpui_dessen::main!(app);
