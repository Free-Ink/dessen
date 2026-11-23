//! Seven-segment numerals, drawn from filled rectangles.

use xpui::{Point, Rect, Renderer};

/// Segments lit for 0 to 9: bit 0 the top, then clockwise, bit 6 the middle.
const SEGMENTS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

/// Paints `value % 10` as a seven-segment digit `width` wide and twice as
/// tall, with strokes `stroke` thick.
pub fn digit(origin: Point, width: i32, stroke: i32, value: u32) {
    let (left, top) = (origin.x, origin.y);
    let thickness = stroke.max(1);
    let lit = SEGMENTS[(value % 10) as usize];
    let segments = [
        Rect::new(left, top, width, thickness),
        Rect::new(left + width - thickness, top, thickness, width),
        Rect::new(left + width - thickness, top + width, thickness, width),
        Rect::new(left, top + 2 * width - thickness, width, thickness),
        Rect::new(left, top + width, thickness, width),
        Rect::new(left, top, thickness, width),
        Rect::new(left, top + width - thickness / 2, width, thickness),
    ];
    for (bit, segment) in segments.into_iter().enumerate() {
        if lit & (1 << bit) != 0 {
            Renderer::fill_rect(segment, true);
        }
    }
}

/// Paints a number in seven-segment digits, `width` wide each, centred on
/// `centre_x`; returns how tall it is.
pub fn number(centre_x: i32, top: i32, width: i32, value: u32) -> i32 {
    let mut digits = [0u32; 10];
    let (mut count, mut rest) = (0, value);
    loop {
        digits[count] = rest % 10;
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    let gap = width / 4;
    let total = count as i32 * width + (count as i32 - 1) * gap;
    let mut left = centre_x - total / 2;
    // Stored least significant first, so they are painted in reverse.
    for &value in digits[..count].iter().rev() {
        digit(Point::new(left, top), width, (width / 5).max(3), value);
        left += width + gap;
    }
    2 * width
}
