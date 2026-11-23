//! Recents, as an xpui home launcher: recently opened books, then recently
//! added ones, each with its cover and how far it is read, over a bar that
//! reaches every other home destination.
//!
//! Tap a book, or move to it with Up and Down and press Confirm. Swipes, or
//! Left and Right, turn the page; Back opens Files. It is the xpui version of
//! `apps/home.c`.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod layout;
mod nav_bar;
mod recents;
mod screen;
mod shelf_view;

use xpui::App;
use xpui::host::Theme;

// Measured on the VM: heap 8576 B at most, stack 1428 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(12032);
dessen::stack!(2560);

fn app() -> App {
    let per_page = layout::books_per_page(Theme::content_area().size.height);
    App::new(recents::Recents::new(per_page))
}

xpui_dessen::main!(app);
