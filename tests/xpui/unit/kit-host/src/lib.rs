//! kit's files that draw nothing, compiled for the host: `join!`, the recents
//! shelf and the status line. Its drawing (digits, lines, text, bitmaps) is
//! checked by the screen tests.

extern crate alloc;

#[path = "../../../../../apps/xpui/kit/src/join.rs"]
mod join;
#[path = "../../../../../apps/xpui/kit/src/shelf.rs"]
pub mod shelf;
#[path = "../../../../../apps/xpui/kit/src/status.rs"]
mod status;

#[doc(hidden)]
pub use alloc::string::String as __String;
pub use join::{Padded, Piece};
pub use status::{battery, clock, status};

#[cfg(test)]
mod tests;
