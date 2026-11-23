//! Lights Out: flipping a square also flips its four neighbours; turn the
//! whole board white. Tap a square, or move the cursor with the keys and flip
//! with Confirm. Every level is solvable: it is built by pressing random
//! squares, starting from a solved board.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod board;
mod screen;

use xpui::App;

// Measured on the VM: heap 632 B at most, stack 1400 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(1280);
dessen::stack!(2560);

fn app() -> App {
    App::new(screen::LightsOut::new())
}

xpui_dessen::main!(app);
