//! dessen's 1-bit images: covers and bundled icons.

use alloc::vec;
use alloc::vec::Vec;
use xpui::{Point, Renderer, Size};

/// A 1-bit image in dessen's format: rows MSB first and padded to a byte, a
/// set bit black.
pub struct Bitmap {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// xpui's rows: the same layout, but a clear bit is ink.
    rows: Vec<u8>,
}

impl Bitmap {
    /// An image `width` × `height` from dessen's bits, or `None` when `bits`
    /// is shorter than that.
    pub fn new(width: i32, height: i32, bits: &[u8]) -> Option<Bitmap> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let len = (width as usize).div_ceil(8) * height as usize;
        let rows = bits.get(..len)?.iter().map(|byte| !byte).collect();
        Some(Bitmap {
            width,
            height,
            rows,
        })
    }

    /// The cover of the book at `path`, `height` pixels tall, read into a
    /// buffer of `max` bytes as `dsn_book_cover` answers it (`{u16 width,
    /// u16 height}`, then the bits); `None` when it has none or the board
    /// cannot decode one.
    ///
    /// The image is made in the buffer it was read into, cut down to its
    /// rows: the heap frees the rest where it is, so a cover never needs room
    /// for two copies of itself.
    ///
    /// The buffer is only as big as a cover three quarters as wide as it is
    /// tall, when that is less than `max`: book covers are narrower (2:3 is
    /// the norm), and a wider one is refused rather than paid for by every
    /// cover. That is a quarter less heap than room for a square.
    pub fn cover(path: &str, height: i32, max: usize) -> Option<Bitmap> {
        const HEADER: usize = 4;
        let tall = usize::try_from(height).ok()?;
        let wide = tall * 3 / 4;
        let mut data = vec![0u8; (HEADER + wide.div_ceil(8) * tall).min(max)];
        let len = dessen::dsn_book_cover(path.as_bytes(), height, &mut data);
        let len = usize::try_from(len).ok()?.min(data.len());
        let [width_low, width_high, height_low, height_high, ..] = data[..len] else {
            return None;
        };
        let width = i32::from(u16::from_le_bytes([width_low, width_high]));
        let height = i32::from(u16::from_le_bytes([height_low, height_high]));
        if width == 0 || height == 0 {
            return None;
        }
        let rows = (width as usize).div_ceil(8) * height as usize;
        if HEADER + rows > len {
            return None;
        }
        data.copy_within(HEADER..HEADER + rows, 0);
        data.truncate(rows);
        data.shrink_to_fit();
        for byte in &mut data {
            *byte = !*byte;
        }
        Some(Bitmap {
            width,
            height,
            rows: data,
        })
    }

    /// Its size.
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Paints it with its top-left corner at `origin`.
    pub fn draw(&self, origin: Point) {
        Renderer::draw_image(origin, &self.rows, self.size());
    }
}
