//! A minimal home launcher, as an xpui list. Each entry in `menu::MENU` opens
//! a firmware screen or an installed dessen app; edit it to build your own
//! home. A recent book adds Continue Reading on top. The header shows the time
//! and the battery, and Back does nothing, since this is the root of the
//! device.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod menu;
mod screen;

use xpui::App;

// Measured on the VM: heap 1064 B at most, stack 1752 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(1792);
dessen::stack!(2816);

fn app() -> App {
    App::new(screen::Home::new())
}

xpui_dessen::main!(app);
