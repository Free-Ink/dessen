use crate::board::{Board, SIDE, SQUARES};

fn lit(board: &Board) -> Vec<usize> {
    (0..SQUARES).filter(|i| board.is_lit(*i)).collect()
}

/// A board with nothing lit: level 1, with the presses that solve it.
fn dark() -> Board {
    let mut board = Board::level(1);
    let presses = solve(&board).expect("level 1 is solvable");
    for (square, press) in presses.iter().enumerate() {
        if *press {
            board.press(square);
        }
    }
    assert!(board.solved());
    board
}

#[test]
fn a_press_flips_its_cross() {
    let mut board = dark();
    board.press(12); // the centre
    assert_eq!(lit(&board), [7, 11, 12, 13, 17]);
    board.press(12);
    assert!(board.solved(), "pressing twice undoes it");
}

#[test]
fn a_press_stops_at_the_edges() {
    let mut board = dark();
    board.press(0);
    assert_eq!(lit(&board), [0, 1, SIDE]);
    board.press(0);
    board.press(SQUARES - 1);
    assert_eq!(lit(&board), [SQUARES - SIDE - 1, SQUARES - 2, SQUARES - 1]);
    board.press(SQUARES - 1);
    board.press(2); // top edge, not a corner
    assert_eq!(lit(&board), [1, 2, 3, 7]);
}

/// The presses that turn `board` dark, by Gaussian elimination over GF(2);
/// `None` when it cannot be done (a 5 × 5 board has unsolvable starts).
fn solve(board: &Board) -> Option<[bool; SQUARES]> {
    // Row i: which presses flip square i, and whether square i is lit.
    let mut rows: Vec<(u32, bool)> = (0..SQUARES)
        .map(|square| {
            let mut effect = 0u32;
            for press in 0..SQUARES {
                let (a, b) = ((square / SIDE) as i32, (square % SIDE) as i32);
                let (c, d) = ((press / SIDE) as i32, (press % SIDE) as i32);
                if (a - c).abs() + (b - d).abs() <= 1 {
                    effect |= 1 << press;
                }
            }
            (effect, board.is_lit(square))
        })
        .collect();
    let mut pivot_of = [usize::MAX; SQUARES];
    let mut next = 0;
    for column in 0..SQUARES {
        let Some(found) = (next..SQUARES).find(|r| rows[*r].0 >> column & 1 == 1) else { continue };
        rows.swap(next, found);
        for r in 0..SQUARES {
            if r != next && rows[r].0 >> column & 1 == 1 {
                rows[r].0 ^= rows[next].0;
                rows[r].1 ^= rows[next].1;
            }
        }
        pivot_of[column] = next;
        next += 1;
    }
    if rows[next..].iter().any(|(effect, light)| *effect == 0 && *light) {
        return None;
    }
    let mut presses = [false; SQUARES];
    for column in 0..SQUARES {
        if pivot_of[column] != usize::MAX {
            presses[column] = rows[pivot_of[column]].1;
        }
    }
    Some(presses)
}

#[test]
fn every_level_is_lit_solvable_and_the_same_each_time() {
    for level in 1..=40 {
        let board = Board::level(level);
        assert!(!board.solved(), "level {level} starts solved");
        assert_eq!(lit(&board), lit(&Board::level(level)), "level {level} changed between builds");
        let presses = solve(&board).unwrap_or_else(|| panic!("level {level} cannot be solved"));
        let mut played = Board::level(level);
        for (square, press) in presses.iter().enumerate() {
            if *press {
                played.press(square);
            }
        }
        assert!(played.solved(), "level {level}: the solution did not solve it");
    }
}

#[test]
fn levels_differ() {
    assert_ne!(lit(&Board::level(1)), lit(&Board::level(2)));
}
