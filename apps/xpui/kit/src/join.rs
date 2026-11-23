//! Short strings built without `core::fmt`.
//!
//! `format!` links Rust's whole formatting machinery, 3 to 5 KB of every image
//! that uses it, and an image is RAM here: the VM loads all of it. The apps
//! only ever put words and whole numbers side by side, so [`join!`] does that
//! and nothing more.
//!
//! ```ignore
//! kit::join!("Level ", level, "   Moves ", moves)   // "Level 3   Moves 12"
//! kit::join!(hour, ":", kit::Padded(minute, 2))      // "9:05"
//! ```

use alloc::string::String;

/// One part of a [`join!`]: words, or a number in decimal.
pub trait Piece {
    /// Appends this part to `out`.
    fn push_to(&self, out: &mut String);
}

impl<T: Piece + ?Sized> Piece for &T {
    fn push_to(&self, out: &mut String) {
        (**self).push_to(out);
    }
}

impl Piece for str {
    fn push_to(&self, out: &mut String) {
        out.push_str(self);
    }
}

impl Piece for String {
    fn push_to(&self, out: &mut String) {
        out.push_str(self);
    }
}

impl Piece for char {
    fn push_to(&self, out: &mut String) {
        out.push(*self);
    }
}

impl Piece for u32 {
    fn push_to(&self, out: &mut String) {
        Padded(*self, 1).push_to(out);
    }
}

impl Piece for usize {
    fn push_to(&self, out: &mut String) {
        Padded(*self as u32, 1).push_to(out);
    }
}

impl Piece for i32 {
    fn push_to(&self, out: &mut String) {
        if *self < 0 {
            out.push('-');
        }
        self.unsigned_abs().push_to(out);
    }
}

/// A number with zeros in front to make at least `.1` digits: `Padded(5, 2)`
/// is "05".
pub struct Padded(pub u32, pub usize);

impl Piece for Padded {
    fn push_to(&self, out: &mut String) {
        let Padded(mut rest, width) = *self;
        // u32::MAX has ten digits.
        let mut digits = [b'0'; 10];
        let mut start = digits.len();
        while rest > 0 || start == digits.len() {
            start -= 1;
            digits[start] = b'0' + (rest % 10) as u8;
            rest /= 10;
        }
        let start = start.min(digits.len().saturating_sub(width));
        for digit in &digits[start..] {
            out.push(char::from(*digit));
        }
    }
}

/// The given parts, one after another, as a `String`: words (`&str`,
/// `String`, `char`) and numbers (`u32`, `usize`, `i32`, [`Padded`]).
#[macro_export]
macro_rules! join {
    ($($piece:expr),+ $(,)?) => {{
        let mut out = $crate::__String::new();
        $( $crate::Piece::push_to(&$piece, &mut out); )+
        out
    }};
}
