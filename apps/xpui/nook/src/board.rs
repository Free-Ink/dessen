//! The shelf: where the books stand, the plants placed so far, and the rules.
//!
//! Every book needs its own plant in the cubby above, below or beside it, no
//! two plants may touch (not even at a corner), and each row and column holds
//! as many plants as its number says. This file uses nothing but `core`, so
//! `make xpui` also compiles it for the desktop and runs the tests below.

/// The widest shelf: its cells fit the bits of a `u64`.
pub const MAX_SIDE: usize = 8;

/// What a cubby holds.
#[derive(Clone, Copy, PartialEq)]
pub enum Content {
    Empty,
    Book,
    Plant,
    /// The player's note that no plant goes here.
    Mark,
}

/// A shelf `side` cubbies square. Cells count along rows from the top left,
/// and `books`, `plants` and `marks` have bit `cell` set for each one.
#[derive(Clone, Copy)]
pub struct Board {
    pub side: usize,
    pub books: u64,
    pub plants: u64,
    pub marks: u64,
    /// Plants each row must hold, and each column.
    pub rows: [u8; MAX_SIDE],
    pub columns: [u8; MAX_SIDE],
}

/// The set bits of `mask`, lowest first.
fn bits(mut mask: u64) -> impl Iterator<Item = usize> {
    core::iter::from_fn(move || {
        (mask != 0).then(|| {
            let bit = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            bit
        })
    })
}

impl Board {
    /// The cells beside `cell`: the four sharing an edge, and with `corners`
    /// the four diagonal ones too.
    fn around(&self, cell: usize, corners: bool) -> u64 {
        const STEPS: [(i32, i32); 8] = [
            (-1, 0),
            (0, -1),
            (0, 1),
            (1, 0),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ];
        let side = self.side as i32;
        let (row, column) = (cell as i32 / side, cell as i32 % side);
        let mut mask = 0;
        for (down, across) in &STEPS[..if corners { 8 } else { 4 }] {
            let (row, column) = (row + down, column + across);
            if (0..side).contains(&row) && (0..side).contains(&column) {
                mask |= 1 << (row * side + column);
            }
        }
        mask
    }

    pub fn row_plants(&self, row: usize) -> u8 {
        let cells = (0..self.side).map(|column| row * self.side + column);
        cells.filter(|cell| self.plants >> cell & 1 != 0).count() as u8
    }

    pub fn column_plants(&self, column: usize) -> u8 {
        let cells = (0..self.side).map(|row| row * self.side + column);
        cells.filter(|cell| self.plants >> cell & 1 != 0).count() as u8
    }

    pub fn content(&self, cell: usize) -> Content {
        let has = |mask: u64| mask >> cell & 1 != 0;
        if has(self.books) {
            Content::Book
        } else if has(self.plants) {
            Content::Plant
        } else if has(self.marks) {
            Content::Mark
        } else {
            Content::Empty
        }
    }

    /// Whether `cell` touches a plant, at an edge or a corner.
    pub fn crowded(&self, cell: usize) -> bool {
        self.around(cell, true) & self.plants != 0
    }

    /// A tap on `cell`: empty, then a plant, then a mark, then empty again.
    /// A book stays.
    pub fn tap(&mut self, cell: usize) {
        let bit = 1 << cell;
        match self.content(cell) {
            Content::Book => {}
            Content::Empty => self.plants |= bit,
            Content::Plant => {
                self.plants &= !bit;
                self.marks |= bit;
            }
            Content::Mark => self.marks &= !bit,
        }
    }

    /// Whether every one of `books` can take a plant of its own from `plants`,
    /// each from a cell sharing an edge with it.
    fn paired(&self, books: u64, plants: u64) -> bool {
        if books == 0 {
            return true;
        }
        let book = books.trailing_zeros() as usize;
        bits(self.around(book, false) & plants)
            .any(|plant| self.paired(books & (books - 1), plants & !(1 << plant)))
    }

    /// Whether the plants placed follow every rule. Checked by the rules, not
    /// against a stored answer.
    pub fn solved(&self) -> bool {
        (0..self.side).all(|lane| {
            self.row_plants(lane) == self.rows[lane] && self.column_plants(lane) == self.columns[lane]
        }) && bits(self.plants).all(|plant| self.around(plant, true) & self.plants == 0)
            && self.paired(self.books, self.plants)
    }

    /// Ways to fill the shelf from empty, counted up to `limit`.
    pub fn solutions(&self, limit: u32) -> u32 {
        let (mut rows, mut columns) = (self.rows, self.columns);
        self.search(0, 0, &mut rows, &mut columns, limit)
    }

    /// Solutions that add plants at `from` or later to `plants`; `rows` and
    /// `columns` are the plants each lane still needs.
    fn search(
        &self,
        from: usize,
        plants: u64,
        rows: &mut [u8; MAX_SIDE],
        columns: &mut [u8; MAX_SIDE],
        limit: u32,
    ) -> u32 {
        // With a plant for every book, every lane's count has reached zero.
        if plants.count_ones() == self.books.count_ones() {
            return u32::from(self.paired(self.books, plants));
        }
        let mut total = 0;
        for cell in from..self.side * self.side {
            let (row, column) = (cell / self.side, cell % self.side);
            // Past a row still short of plants there is nothing to find.
            if total >= limit || (row > 0 && rows[row - 1] != 0) {
                break;
            }
            let open = self.books >> cell & 1 == 0
                && self.around(cell, true) & plants == 0
                && self.around(cell, false) & self.books != 0;
            if open && rows[row] > 0 && columns[column] > 0 {
                rows[row] -= 1;
                columns[column] -= 1;
                total += self.search(cell + 1, plants | 1 << cell, rows, columns, limit - total);
                rows[row] += 1;
                columns[column] += 1;
            }
        }
        total
    }
}

/// A planted shelf with exactly one way to plant it: plants dropped on random
/// cells that touch no other plant, each with a book put beside it, tried
/// again until the numbers allow no second answer. `below(count)` gives a
/// random number under `count`.
pub fn shelf(side: usize, below: &mut impl FnMut(usize) -> usize) -> Board {
    let cells = side * side;
    loop {
        let mut board = Board {
            side,
            books: 0,
            plants: 0,
            marks: 0,
            rows: [0; MAX_SIDE],
            columns: [0; MAX_SIDE],
        };
        let mut order: [u8; MAX_SIDE * MAX_SIDE] = core::array::from_fn(|cell| cell as u8);
        for index in (1..cells).rev() {
            order.swap(index, below(index + 1));
        }
        for cell in order[..cells].iter().map(|cell| usize::from(*cell)) {
            // A plant in about one cubby of five.
            if board.plants.count_ones() as usize >= cells / 5 {
                break;
            }
            let taken = board.books | board.plants;
            let spots = board.around(cell, false) & !taken;
            if taken >> cell & 1 != 0 || board.around(cell, true) & board.plants != 0 {
                continue;
            }
            if let Some(book) = bits(spots).nth(below(spots.count_ones().max(1) as usize)) {
                board.plants |= 1 << cell;
                board.books |= 1 << book;
            }
        }
        for lane in 0..side {
            board.rows[lane] = board.row_plants(lane);
            board.columns[lane] = board.column_plants(lane);
        }
        if board.solutions(2) == 1 {
            return board;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 × 3 shelf with books in the middle of the top and bottom rows.
    fn small(plants: u64, rows: [u8; 3], columns: [u8; 3]) -> Board {
        let mut board = Board {
            side: 3,
            books: 1 << 1 | 1 << 7,
            plants,
            marks: 0,
            rows: [0; MAX_SIDE],
            columns: [0; MAX_SIDE],
        };
        board.rows[..3].copy_from_slice(&rows);
        board.columns[..3].copy_from_slice(&columns);
        board
    }

    #[test]
    fn rules() {
        assert!(small(1 << 0 | 1 << 6, [1, 0, 1], [2, 0, 0]).solved());
        // Plants touching at a corner.
        assert!(!small(1 << 0 | 1 << 4, [1, 1, 0], [1, 1, 0]).solved());
        // Both plants beside the top book, none for the bottom one.
        assert!(!small(1 << 0 | 1 << 2, [2, 0, 0], [1, 0, 1]).solved());
        // The numbers not met.
        assert!(!small(1 << 0 | 1 << 6, [1, 0, 1], [1, 1, 0]).solved());
    }

    #[test]
    fn generated_shelves_have_one_answer() {
        let mut seed = 1u32;
        let mut below = |count: usize| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((u64::from(seed) * count as u64) >> 32) as usize
        };
        for round in 0..60 {
            let side = 6 + round % 3;
            let mut board = shelf(side, &mut below);
            assert!(board.solved());
            assert_eq!(board.solutions(2), 1);
            assert_eq!(board.plants.count_ones(), board.books.count_ones());
            assert!(board.plants != 0);
            assert!(bits(board.plants).all(|plant| !board.crowded(plant)));
            board.plants &= board.plants - 1;
            assert!(!board.solved());
            // Taps go round: plant, mark, empty; a book stays.
            let cell = bits(!(board.books | board.plants)).next().unwrap();
            for want in [Content::Plant, Content::Mark, Content::Empty] {
                board.tap(cell);
                assert!(board.content(cell) == want);
            }
            let book = board.books.trailing_zeros() as usize;
            board.tap(book);
            assert!(board.content(book) == Content::Book);
        }
    }
}
