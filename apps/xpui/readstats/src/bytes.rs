//! A fixed buffer of bytes, filled by appending.

/// Up to `N` bytes, appended in pieces; a piece that does not fit is refused
/// whole.
pub struct Bytes<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Bytes<N> {
    pub fn new() -> Self {
        Bytes {
            data: [0; N],
            len: 0,
        }
    }

    /// Appends `piece`; false, changing nothing, when it does not fit.
    pub fn push(&mut self, piece: &[u8]) -> bool {
        let end = self.len + piece.len();
        if end > N {
            return false;
        }
        self.data[self.len..end].copy_from_slice(piece);
        self.len = end;
        true
    }

    /// Appends every piece in turn; false at the first that does not fit.
    pub fn push_all(&mut self, pieces: &[&[u8]]) -> bool {
        pieces.iter().all(|piece| self.push(piece))
    }

    /// Drops trailing bytes equal to `byte`.
    pub fn trim_end(&mut self, byte: u8) {
        while self.len > 0 && self.data[self.len - 1] == byte {
            self.len -= 1;
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
