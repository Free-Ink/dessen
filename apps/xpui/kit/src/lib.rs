//! What the xpui ports of dessen's apps share.
//!
//! - [`digit`] and [`number`] draw seven-segment numerals, so a result reads
//!   large on any board without a big font.
//! - [`step`] moves a cursor over a board of cells (an `xpui::CellGrid`) with
//!   the keys, for the puzzles.
//! - [`Face`], [`text`] and [`paragraph`] place text in a box in any of the
//!   firmware's three faces.
//! - [`Bitmap`] paints dessen's 1-bit images (a cover, an icon), [`shelf`]
//!   reads the recents shelf, and [`status`] builds a home screen's title line,
//!   for the home launchers.
//! - [`join!`] puts words and numbers into a `String` without `core::fmt`,
//!   which `format!` would add to every image.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod bitmap;
mod cursor;
mod digits;
mod join;
mod lines;
pub mod shelf;
mod status;
mod text;

#[doc(hidden)]
pub use alloc::string::String as __String;
pub use bitmap::Bitmap;
pub use cursor::step;
pub use digits::{digit, number};
pub use join::{Padded, Piece};
pub use lines::{frame, thick_line};
pub use status::{battery, clock, status};
pub use text::{Align, Face, fit, paragraph, text};
