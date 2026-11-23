//! The dice, and what has been rolled.

use alloc::string::String;

/// Sides of each die, in the order they are shown.
pub const SIDES: [u32; 7] = [4, 6, 8, 10, 12, 20, 100];
/// Each die's name.
pub const NAMES: [&str; 7] = ["d4", "d6", "d8", "d10", "d12", "d20", "d100"];
/// How many past rolls are kept.
const HISTORY: usize = 5;

/// The die picked, its last roll, and the rolls before it.
pub struct Dice {
    pub selected: usize,
    /// The last roll; 0 before the selected die has been rolled.
    pub rolled: u32,
    /// Past rolls as (die, value), newest first; `kept` of them are filled.
    history: [(usize, u32); HISTORY],
    kept: usize,
}

impl Dice {
    pub fn new(selected: usize) -> Self {
        Dice {
            selected,
            rolled: 0,
            history: [(0, 0); HISTORY],
            kept: 0,
        }
    }

    /// Picks die `index`, clearing the last roll.
    pub fn select(&mut self, index: usize) {
        self.selected = index;
        self.rolled = 0;
    }

    /// Rolls the selected die.
    pub fn roll(&mut self) {
        // A 32-bit random number scaled into 1..=sides, without modulo bias.
        let sides = u64::from(SIDES[self.selected]);
        self.rolled = ((u64::from(dessen::dsn_random()) * sides) >> 32) as u32 + 1;
        self.history.copy_within(0..HISTORY - 1, 1);
        self.history[0] = (self.selected, self.rolled);
        self.kept = (self.kept + 1).min(HISTORY);
    }

    /// "Rolled d20: critical hit!"
    pub fn verdict(&self) -> String {
        let critical = match (SIDES[self.selected], self.rolled) {
            (20, 20) => ": critical hit!",
            (20, 1) => ": critical fail.",
            _ => "",
        };
        kit::join!("Rolled ", NAMES[self.selected], critical)
    }

    /// "Last:  d20=17  d6=3"
    pub fn history(&self) -> String {
        let mut line = String::from("Last:");
        for (die, value) in &self.history[..self.kept] {
            line.push_str(&kit::join!("  ", NAMES[*die], "=", *value));
        }
        line
    }
}
