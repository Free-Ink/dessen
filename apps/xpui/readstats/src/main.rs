//! Reading stats, as an xpui home launcher: cards built from the firmware's
//! reading session log (`/.crosspoint/reading_sessions.jsonl`, one JSON object
//! per line; see crosspoint-reader `src/sync/ReadingSessionLog.h`). The log is
//! never trimmed, so it is streamed through a small buffer, not loaded whole.
//!
//! The book being read is a card pinned to the bottom of page 1, and every
//! later page ends in a grid of icons for the firmware's own screens. The
//! cards flow top to bottom and spill onto further pages, so the same code
//! fits any panel. Swipes, or a tap on empty space, turn the page. Up and Down
//! step a focus through the book card, then each icon on page 2, then the
//! remaining pages; Confirm or a tap opens what is focused or touched. Back
//! returns to page 1, and on page 1 leaves.
//!
//! With an empty log and a CrossPoint Sync account set up, page 1 offers to
//! import the account's history. It is the xpui version of `apps/readstats.rs`.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

mod app_grid;
mod book_card;
mod bytes;
mod cards;
mod clock;
mod focus;
mod format;
mod glyphs;
mod json;
mod layout;
mod log;
mod model;
mod page_view;
mod paint;
mod screen;
mod sync;

use xpui::App;

// Measured on the VM: heap 6608 B at most, stack 4844 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(9472);
dessen::stack!(6144);

fn app() -> App {
    let clock = clock::Clock::read();
    let stats = log::Stats::new(clock.today);
    stats.load(clock.valid);

    // Import only into a missing or empty log: one with any lines is this
    // reader's own history.
    let account = if stats.sessions == 0 && stats.error == 0 && log::is_empty() {
        sync::Account::load()
    } else {
        None
    };
    let model = model::Model::new(stats, account.is_some());
    let focus_ring = xpui_dessen::Device::read().has_buttons;

    let screen = screen::ReadStats::new(model, account, clock, focus_ring);

    App::new(screen)
}

xpui_dessen::main!(app);
