use crate::board::{Board, CELLS, column, row, square};
use crate::generate;

#[test]
fn cells_map_to_their_lanes() {
    assert_eq!((row(0), column(0), square(0)), (0, 0, 0));
    assert_eq!((row(40), column(40), square(40)), (4, 4, 4));
    assert_eq!((row(80), column(80), square(80)), (8, 8, 8));
    assert_eq!((row(26), column(26), square(26)), (2, 8, 2));
    assert_eq!((row(27), column(27), square(27)), (3, 0, 3));
}

/// Every row, column and box holds 1 to 9 once.
fn valid(grid: &[u8; CELLS]) -> bool {
    (0..9).all(|lane| {
        [row as fn(usize) -> usize, column, square].iter().all(|of| {
            let mut seen = 0u16;
            for cell in (0..CELLS).filter(|c| of(*c) == lane) {
                seen |= 1 << grid[cell];
            }
            seen == 0b11_1111_1110
        })
    })
}

#[test]
fn an_empty_board_has_many_solutions_and_a_full_one_one() {
    let mut board = Board::new([0; CELLS]);
    assert_eq!(board.solutions(2), 2, "counting stops at the limit");
    let solution: [u8; CELLS] = core::array::from_fn(|c| ((row(c) * 3 + row(c) / 3 + column(c)) % 9) as u8 + 1);
    assert!(valid(&solution));
    let mut full = Board::new(solution);
    for cell in 0..CELLS {
        full.set(cell, solution[cell]);
    }
    assert_eq!(full.solutions(2), 1);
    assert!(full.solved());
    assert_eq!(full.empty(), 0);
}

#[test]
fn a_contradiction_has_no_solution() {
    // A solved grid with two cells of the top row taken out, and the first
    // one's digit put in the second: the first cell has nothing left to take.
    let solution: [u8; CELLS] = core::array::from_fn(|c| ((row(c) * 3 + row(c) / 3 + column(c)) % 9) as u8 + 1);
    let mut board = Board::new(solution);
    for cell in 2..CELLS {
        board.set(cell, solution[cell]);
    }
    board.set(1, solution[0]);
    assert_eq!(board.solutions(2), 0);
}

#[test]
fn set_keeps_the_masks_when_a_digit_is_replaced_or_cleared() {
    let mut board = Board::new([0; CELLS]);
    board.set(0, 5);
    board.set(0, 6);
    board.set(1, 5); // legal now that 5 left cell 0
    board.set(0, 0);
    assert_eq!(board.empty(), CELLS - 1);
    assert!(board.solutions(1) >= 1);
}

fn puzzle(seed: u32) -> Board {
    dessen::reset(|w| w.seed = seed);
    generate::puzzle()
}

#[test]
fn every_puzzle_has_exactly_one_solution() {
    for seed in [1, 7, 42, 0xdead_beef, 0x9e37_79b9] {
        let board = puzzle(seed);
        assert!(valid(&board.solution), "seed {seed}: the solution is a sudoku");
        let clues = (0..CELLS).filter(|c| board.given[*c]).count();
        assert!((17..=40).contains(&clues), "seed {seed}: {clues} clues");
        assert_eq!(board.empty(), CELLS - clues);
        for cell in 0..CELLS {
            assert_eq!(board.given[cell], board.grid[cell] != 0);
            if board.given[cell] {
                assert_eq!(board.grid[cell], board.solution[cell], "seed {seed}: a clue that is not the solution");
            }
        }
        let mut copy = Board::new(board.solution);
        for cell in (0..CELLS).filter(|c| board.given[*c]) {
            copy.set(cell, board.grid[cell]);
        }
        assert_eq!(copy.solutions(2), 1, "seed {seed}: not unique");
        assert!(!board.solved());
    }
}

#[test]
fn the_same_seed_makes_the_same_puzzle() {
    let (a, b) = (puzzle(7), puzzle(7));
    assert_eq!(a.grid, b.grid);
    assert_ne!(puzzle(8).grid, a.grid);
}

#[test]
fn filling_in_the_solution_solves_it() {
    let mut board = puzzle(3);
    let open: Vec<usize> = (0..CELLS).filter(|c| !board.given[*c]).collect();
    for cell in open {
        assert!(!board.solved());
        board.set(cell, board.solution[cell]);
    }
    assert!(board.solved());
    assert_eq!(board.empty(), 0);
}
