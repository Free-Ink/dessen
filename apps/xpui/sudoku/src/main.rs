//! Sudoku. Every puzzle is generated on the device and has exactly one
//! solution. Tap a cell, then a number below the board (X erases). With keys:
//! move the cursor, and Confirm steps the cell through 1 to 9 and blank.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod board;
mod generate;
mod paint;
mod screen;

use xpui::App;

// Measured on the VM: heap 1040 B at most, stack 4388 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(1792);
dessen::stack!(5632);

fn app() -> App {
    // Generating takes a moment on the VM, so say so before xpui paints.
    dessen::dsn_clear();
    let message = b"Sudoku: making a puzzle...";
    let (font, align) = (dessen::DSN_FONT_BODY, dessen::DSN_ALIGN_LEFT);
    dessen::dsn_text_box(16, 12, 0, 0, message, font, align, 1, 0);
    dessen::dsn_refresh();
    App::new(screen::Sudoku::new(generate::puzzle()))
}

xpui_dessen::main!(app);
