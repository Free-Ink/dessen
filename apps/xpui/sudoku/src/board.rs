//! The board: the digits placed, the solution, and a solver that counts
//! solutions.

/// Cells on the board.
pub const CELLS: usize = 81;

/// The row of `cell`, counting cells along rows from the top left.
pub const fn row(cell: usize) -> usize {
    cell / 9
}

/// The column of `cell`.
pub const fn column(cell: usize) -> usize {
    cell % 9
}

/// The 3 × 3 box of `cell`, numbered along rows of boxes.
pub const fn square(cell: usize) -> usize {
    row(cell) / 3 * 3 + column(cell) / 3
}

/// The bit standing for `digit` (1 to 9) in a lane's mask.
const fn bit(digit: u8) -> u16 {
    1 << (digit - 1)
}

pub struct Board {
    /// The digit in each cell; 0 is empty.
    pub grid: [u8; CELLS],
    pub solution: [u8; CELLS],
    /// Whether each cell is a printed clue, which cannot be changed.
    pub given: [bool; CELLS],
    /// For rows, columns and boxes in turn: bit `digit - 1` set when that
    /// digit is placed in the lane.
    used: [[u16; 9]; 3],
}

impl Board {
    /// An empty board that will be filled toward `solution`.
    pub fn new(solution: [u8; CELLS]) -> Board {
        Board {
            grid: [0; CELLS],
            solution,
            given: [false; CELLS],
            used: [[0; 9]; 3],
        }
    }

    /// Puts `digit` (0 to clear) in `cell`, keeping the lanes' masks.
    pub fn set(&mut self, cell: usize, digit: u8) {
        let lanes = [row(cell), column(cell), square(cell)];
        let old = self.grid[cell];
        for (kind, lane) in lanes.into_iter().enumerate() {
            if old != 0 {
                self.used[kind][lane] &= !bit(old);
            }
            if digit != 0 {
                self.used[kind][lane] |= bit(digit);
            }
        }
        self.grid[cell] = digit;
    }

    /// The digits `cell` could still take, as a mask of bits.
    fn candidates(&self, cell: usize) -> u16 {
        let used =
            self.used[0][row(cell)] | self.used[1][column(cell)] | self.used[2][square(cell)];
        !used & 0x1ff
    }

    /// Solutions of the current grid, counted up to `limit`. Fills the cell
    /// with the fewest candidates first, which keeps the search small.
    pub fn solutions(&mut self, limit: u32) -> u32 {
        let mut best = None;
        let mut fewest = 10;
        for cell in (0..CELLS).filter(|cell| self.grid[*cell] == 0) {
            let count = self.candidates(cell).count_ones();
            if count < fewest {
                best = Some(cell);
                fewest = count;
            }
            if count <= 1 {
                break;
            }
        }
        let Some(cell) = best else { return 1 };
        let candidates = self.candidates(cell);
        let mut total = 0;
        for digit in 1..=9u8 {
            if total >= limit {
                break;
            }
            if candidates & bit(digit) != 0 {
                self.set(cell, digit);
                total += self.solutions(limit - total);
                self.set(cell, 0);
            }
        }
        total
    }

    /// Cells still empty.
    pub fn empty(&self) -> usize {
        self.grid.iter().filter(|digit| **digit == 0).count()
    }

    pub fn solved(&self) -> bool {
        self.grid == self.solution
    }
}
