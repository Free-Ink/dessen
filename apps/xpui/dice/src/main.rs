//! Polyhedral dice for tabletop games. Tap a die to roll it, tap the result
//! to roll the same die again. With keys: Up and Down pick a die, Confirm
//! rolls.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod art;
mod dice;
mod readout;
mod screen;

use xpui::App;

// Measured on the VM: heap 624 B at most, stack 1464 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(1280);
dessen::stack!(2560);

fn app() -> App {
    App::new(screen::DiceScreen::new())
}

xpui_dessen::main!(app);
