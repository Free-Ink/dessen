//! The game: a square board of lights and what pressing one does.

/// Squares along each side.
pub const SIDE: usize = 5;
/// Squares on the board.
pub const SQUARES: usize = SIDE * SIDE;

/// Which squares are lit, counting along rows from the top left.
pub struct Board {
    lit: [bool; SQUARES],
}

impl Board {
    /// The board for `level`: the same one every time, never already solved.
    pub fn level(level: u32) -> Board {
        let presses = (3 + level).min(12);
        let mut seed = level.wrapping_mul(2_654_435_761);
        let mut random = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 16) as usize
        };
        loop {
            let mut board = Board {
                lit: [false; SQUARES],
            };
            for _ in 0..presses {
                let (row, column) = (random() % SIDE, random() % SIDE);
                board.press(row * SIDE + column);
            }
            if !board.solved() {
                return board;
            }
        }
    }

    /// Whether the square at `index` is lit.
    pub fn is_lit(&self, index: usize) -> bool {
        self.lit[index]
    }

    /// Flips the square at `index` and its four neighbours.
    pub fn press(&mut self, index: usize) {
        let (row, column) = ((index / SIDE) as i32, (index % SIDE) as i32);
        for (down, across) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (row, column) = (row + down, column + across);
            let on_board = 0..SIDE as i32;
            if on_board.contains(&row) && on_board.contains(&column) {
                self.lit[row as usize * SIDE + column as usize] ^= true;
            }
        }
    }

    /// Whether every square is dark.
    pub fn solved(&self) -> bool {
        self.lit.iter().all(|lit| !lit)
    }
}
