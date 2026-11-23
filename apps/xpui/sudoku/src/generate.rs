//! Making a puzzle with exactly one solution.

use crate::board::{Board, CELLS, column, row};

/// Clues to stop at: fewer makes a harder puzzle and a longer wait.
const TARGET_CLUES: usize = 30;

/// A number in `0..count`, from the device's random source.
fn below(count: usize) -> usize {
    ((u64::from(dessen::dsn_random()) * count as u64) >> 32) as usize
}

fn shuffle(items: &mut [u8]) {
    for index in (1..items.len()).rev() {
        items.swap(index, below(index + 1));
    }
}

/// A shuffled order of the nine rows (or columns) that keeps a solved grid
/// valid: the three bands are shuffled, and the three lines within each.
fn line_order() -> [u8; 9] {
    let mut order = [0u8; 9];
    let mut bands = [0, 1, 2];
    shuffle(&mut bands);
    for (position, band) in bands.into_iter().enumerate() {
        let mut within = [0, 1, 2];
        shuffle(&mut within);
        for (offset, line) in within.into_iter().enumerate() {
            order[position * 3 + offset] = band * 3 + line;
        }
    }
    order
}

/// A solved grid: a fixed pattern with its digits shuffled, and its rows and
/// columns shuffled only in ways that keep it valid.
fn solved_grid() -> [u8; CELLS] {
    let mut digits = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    shuffle(&mut digits);
    let (rows, columns) = (line_order(), line_order());
    core::array::from_fn(|cell| {
        let row = usize::from(rows[row(cell)]);
        let column = usize::from(columns[column(cell)]);
        digits[(row * 3 + row / 3 + column) % 9]
    })
}

/// A new puzzle: a solved grid with cells taken away in a random order, each
/// removal kept only while the puzzle still has a single solution.
pub fn puzzle() -> Board {
    let mut board = Board::new(solved_grid());
    for cell in 0..CELLS {
        board.set(cell, board.solution[cell]);
    }
    let mut order: [u8; CELLS] = core::array::from_fn(|cell| cell as u8);
    shuffle(&mut order);
    let mut clues = CELLS;
    for cell in order.map(usize::from) {
        if clues <= TARGET_CLUES {
            break;
        }
        board.set(cell, 0);
        if board.solutions(2) == 1 {
            clues -= 1;
        } else {
            board.set(cell, board.solution[cell]);
        }
    }
    board.given = core::array::from_fn(|cell| board.grid[cell] != 0);
    board
}
