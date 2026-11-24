//! Drawing: the bookcase and its boards, the books and plants standing in its
//! cubbies, the number plaques along the top and left, and the buttons.
//!
//! Most of it uses dessen's own drawing calls (triangles, rounded boxes, wide
//! lines, text in a box), which xpui's renderer does not have; both use the
//! same coordinates. docs/drawing-icons.md describes the style.

use xpui::{Point, Rect, Renderer};

use crate::board::Content;

const INK: i32 = dessen::DSN_BLACK;
const PAPER: i32 = dessen::DSN_WHITE;
const ROUND: i32 = dessen::DSN_CORNERS_ALL;

/// One line of `text` in `rect`, which must be a line high at least: centred
/// down it, and across it unless `left`. `flags` are dessen's text flags.
pub fn label(rect: Rect, text: &str, face: i32, left: bool, flags: i32) {
    let align = if left { dessen::DSN_ALIGN_LEFT } else { dessen::DSN_ALIGN_CENTER };
    let (x, y, w, h) = (rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
    dessen::dsn_text_box(x, y, w, h, text.as_bytes(), face, align, 1, flags);
}

/// A box with rounded ends: filled with ink, or paper with an ink edge.
fn capsule(rect: Rect, filled: bool) {
    let (x, y, w, h) = (rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
    let radius = (h / 2).min(w / 2);
    dessen::dsn_fill_round(x, y, w, h, if filled { INK } else { PAPER }, radius, ROUND);
    if !filled {
        dessen::dsn_stroke(x, y, w, h, INK, 2, radius, ROUND);
    }
}

/// A button: `text` in a capsule, white on ink when `filled`.
pub fn button(rect: Rect, text: &str, filled: bool) {
    capsule(rect, filled);
    let invert = if filled { dessen::DSN_TEXT_INVERTED } else { 0 };
    label(rect, text, dessen::DSN_FONT_BODY, false, dessen::DSN_TEXT_BOLD | invert);
}

/// The arrow back, at the left of `rect` and half way down it: three wide
/// strokes, which read better at this size than a thin chevron.
pub fn back_arrow(rect: Rect) {
    let reach = rect.size.height / 2;
    let head = reach / 2;
    let (tip, middle) = (rect.origin.x + 2, rect.origin.y + rect.size.height / 2);
    dessen::dsn_line(tip, middle, tip + reach, middle, 3, INK);
    dessen::dsn_line(tip, middle, tip + head, middle - head, 3, INK);
    dessen::dsn_line(tip, middle, tip + head, middle + head, 3, INK);
}

/// How a row or column stands against its number.
#[derive(Clone, Copy, PartialEq)]
pub enum Count {
    Short,
    Met,
    Over,
}

/// A row or column's number on a plaque: outlined while the lane is short of
/// plants, solid once it holds that many, and grey when it holds too many.
pub fn clue(plaque: Rect, value: u8, count: Count) {
    let (x, y, w, h) = (plaque.origin.x, plaque.origin.y, plaque.size.width, plaque.size.height);
    let radius = h / 4;
    let fill = match count {
        Count::Short => PAPER,
        Count::Met => INK,
        Count::Over => dessen::DSN_LIGHT_GRAY,
    };
    dessen::dsn_fill_round(x, y, w, h, fill, radius, ROUND);
    if count != Count::Met {
        dessen::dsn_stroke(x, y, w, h, INK, 2, radius, ROUND);
    }
    let invert = if count == Count::Met { dessen::DSN_TEXT_INVERTED } else { 0 };
    let digits = kit::join!(u32::from(value));
    label(plaque, &digits, dessen::DSN_FONT_BODY, false, dessen::DSN_TEXT_BOLD | invert);
}

/// A wooden board: grey between two ink edges, which run along it, with a
/// pale line where the light catches its upper edge.
fn plank(rect: Rect, upright: bool) {
    Renderer::fill_rect(rect, true);
    let (grain, across) = if upright { (0, 1) } else { (1, 0) };
    let inner = Rect::new(
        rect.origin.x + across,
        rect.origin.y + grain,
        rect.size.width - 2 * across,
        rect.size.height - 2 * grain,
    );
    if inner.size.width > 0 && inner.size.height > 0 {
        Renderer::fill_rect_dither(inner, false);
    }
    let (wide, tall) = if upright { (1, rect.size.height) } else { (rect.size.width, 1) };
    if inner.size.width > 3 && inner.size.height > 3 {
        Renderer::fill_rect(Rect::new(inner.origin.x, inner.origin.y, wide, tall), false);
    }
}

/// The boards of the case round its cubbies.
pub struct Case {
    /// The side panels' and top board's thickness.
    pub panel: i32,
    /// The moulding over the top board.
    pub crown: i32,
    /// The base the case stands on.
    pub plinth: i32,
    /// How far the crown and the plinth stand out past the sides.
    pub lip: i32,
}

impl Case {
    /// A case for a screen `width` wide.
    pub fn new(width: i32) -> Case {
        let panel = (width / 50).max(4);
        Case {
            panel,
            crown: panel.max(5),
            plinth: panel * 2 + 2,
            lip: (panel / 2).max(3),
        }
    }

    /// Paints the case round `grid`, the rectangle of its cubbies.
    pub fn paint(&self, grid: Rect) {
        let (left, top, wide, tall) = (grid.origin.x, grid.origin.y, grid.size.width, grid.size.height);
        let (panel, lip) = (self.panel, self.lip);
        let (outer_left, outer_wide) = (left - panel, wide + 2 * panel);
        plank(Rect::new(outer_left, top, panel, tall), true);
        plank(Rect::new(left + wide, top, panel, tall), true);
        plank(Rect::new(outer_left, top - panel, outer_wide, panel), false);
        // The crown: an ink moulding with one pale groove.
        let crown = Rect::new(outer_left - lip, top - panel - self.crown, outer_wide + 2 * lip, self.crown);
        Renderer::fill_rect(crown, true);
        let groove = Rect::new(crown.origin.x + 2, crown.origin.y + self.crown - 3, crown.size.width - 4, 1);
        Renderer::fill_rect(groove, false);
        // The plinth, with an arch cut from it so the case stands on two feet.
        let base = top + tall;
        plank(Rect::new(outer_left - lip, base, outer_wide + 2 * lip, self.plinth), false);
        let (arch_left, arch_wide, arch_top) = (left + wide / 6, wide * 2 / 3, base + self.plinth / 3);
        let radius = self.plinth / 2;
        dessen::dsn_fill_round(arch_left, arch_top, arch_wide, self.plinth, PAPER, radius, ROUND);
        dessen::dsn_stroke(arch_left, arch_top, arch_wide, self.plinth, INK, 1, radius, ROUND);
        let below = Rect::new(arch_left - 1, base + self.plinth, arch_wide + 2, self.plinth);
        Renderer::fill_rect(below, false);
    }
}

/// The next number of a small generator, so a cubby's books differ from the
/// next one's but come out the same every time the screen is drawn.
fn next(seed: &mut u32) -> i32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as i32
}

/// One book seen spine on, in one of three bindings.
fn spine(rect: Rect, binding: i32) {
    let (left, top, width, height) = (rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
    let band = (height / 14).max(2);
    let across = |down: i32, tall: i32, black: bool| {
        Renderer::fill_rect(Rect::new(left + 1, top + down, width - 2, tall), black);
    };
    match binding {
        // Dark cloth: two pale bands at the head and a pale title label.
        0 => {
            Renderer::fill_rect(rect, true);
            across(band, 1, false);
            across(band * 2 + 1, 1, false);
            if width >= 6 {
                let label = Rect::new(left + 2, top + height / 3, width - 4, height / 4);
                Renderer::fill_rect(label, false);
            }
        }
        // Pale paper: dark bands at head and foot and a title down the spine.
        1 => {
            Renderer::fill_rect(rect, false);
            Renderer::stroke_rect(rect);
            across(band, band, true);
            across(height - 2 * band, band, true);
            let title = Rect::new(left + width / 2 - 1, top + height / 3, 2, height / 3);
            Renderer::fill_rect(title, true);
        }
        // Grey leather: a raised band at head and foot.
        _ => {
            Renderer::fill_rect(rect, true);
            Renderer::fill_rect_dither(
                Rect::new(left + 1, top + 1, width - 2, height - 2),
                true,
            );
            across(band * 2, band, true);
            across(height - 3 * band, band, true);
        }
    }
}

/// Three books lying flat, one on another, from `left` across `width`.
fn stack(left: i32, floor: i32, width: i32, height: i32, seed: &mut u32) {
    let thick = (height / 7).max(4);
    for level in 0..3 {
        let long = width - 2 - next(seed) % (width / 4).max(1);
        let book = Rect::new(
            left + 1 + next(seed) % (width - long).max(1),
            floor - (level + 1) * (thick - 1) - 1,
            long,
            thick,
        );
        Renderer::fill_rect(book, true);
        let inner = Rect::new(book.origin.x + 1, book.origin.y + 1, long - 2, thick - 2);
        match level {
            0 => Renderer::fill_rect_dither(inner, true),
            1 => Renderer::fill_rect(inner, false),
            _ => {}
        }
        // The band near the spine's end.
        let band = Rect::new(book.origin.x + long / 5, book.origin.y + 1, 2, thick - 2);
        Renderer::fill_rect(band, level == 1);
    }
}

/// What a book cubby holds between `left` and `right` under `top`: a row of
/// books standing on the shelf at `floor`, some with a book leaning on the
/// row or a stack lying flat beside it.
fn books(left: i32, right: i32, top: i32, floor: i32, mut seed: u32) {
    let (width, height) = (right - left, floor - top);
    let thinnest = (width / 9).max(4);
    let beside = next(&mut seed) % 3;
    let room = [width / 4, width * 2 / 5, 0][beside as usize];
    // The row takes one end of the cubby and the other thing the far end.
    let row_first = next(&mut seed) % 2 == 0;
    let (mut at, end, item) = if row_first {
        (left, right - room, right - room)
    } else {
        (left + room, right, left)
    };
    let mut binding = next(&mut seed) % 3;
    loop {
        let thick = thinnest + next(&mut seed) % thinnest;
        if at + thick > end {
            break;
        }
        let tall = height * 13 / 20 + next(&mut seed) % (height * 7 / 20);
        // Never two of a binding together, so each spine stands apart.
        binding = (binding + 1 + next(&mut seed) % 2) % 3;
        spine(Rect::new(at, floor - tall, thick, tall), binding);
        // Neighbours share an edge.
        at += thick - 1;
    }
    match beside {
        0 => {
            // Its head rests on the row, its foot out on the shelf.
            let (head, lean) = if row_first {
                (at + thinnest / 2, room * 2 / 3)
            } else {
                (left + room - thinnest / 2, -room * 2 / 3)
            };
            kit::thick_line(
                Point::new(head, floor - height * 3 / 5),
                Point::new(head + lean, floor - 1),
                thinnest,
            );
        }
        1 => stack(item, floor, room, height, &mut seed),
        _ => {}
    }
}

/// A filled circle of `radius` around a point.
fn disc(across: i32, down: i32, radius: i32, color: i32) {
    let wide = 2 * radius + 1;
    dessen::dsn_fill_round(across - radius, down - radius, wide, wide, color, radius, dessen::DSN_CORNERS_ALL);
}

/// A pointed leaf from `base` to its tip `across` and `up` from there,
/// `half` wide either side at its widest, edged in paper so that it stands
/// apart from the leaves behind it.
fn leaf(base: Point, across: i32, up: i32, half: i32) {
    let tip = Point::new(base.x + across, base.y - up);
    // The length, near enough, without a square root.
    let (long, short) = (across.abs().max(up), across.abs().min(up));
    let length = (long + short * 3 / 8).max(1);
    for (half, color) in [(half + 2, PAPER), (half, INK)] {
        let (side_x, side_y) = (up * half / length, across * half / length);
        let (mid_x, mid_y) = (base.x + across * 2 / 5, base.y - up * 2 / 5);
        for way in [-1, 1] {
            let (x, y) = (mid_x + way * side_x, mid_y + way * side_y);
            dessen::dsn_triangle(base.x, base.y, x, y, tip.x, tip.y, color);
        }
    }
    if half >= 3 {
        let (from_x, from_y) = (base.x + across / 4, base.y - up / 4);
        let (to_x, to_y) = (base.x + across * 3 / 4, base.y - up * 3 / 4);
        dessen::dsn_line(from_x, from_y, to_x, to_y, 1, PAPER);
    }
}

/// A squat ceramic pot `width` wide and `height` tall on the shelf at
/// `floor`: pale and speckled, with a clay band round its foot.
fn pot(centre: i32, floor: i32, width: i32, height: i32) {
    let (left, top) = (centre - width / 2, floor - height);
    let radius = (height / 7).max(2);
    let band = height * 3 / 10;
    let corners = dessen::DSN_CORNERS_ALL;
    dessen::dsn_fill_round(left, top, width, height, dessen::DSN_LIGHT_GRAY, radius, corners);
    Renderer::fill_rect(Rect::new(left, top, width, height - band), false);
    // Speckles in the glaze, in twentieths of the pale part.
    for (across, down) in [(3, 7), (6, 14), (9, 5), (12, 12), (15, 8), (17, 15), (5, 3)] {
        let (x, y) = (left + width * across / 20, top + (height - band) * down / 20);
        Renderer::fill_rect(Rect::new(x, y, 1, 1), true);
    }
    dessen::dsn_line(left + 1, floor - band, left + width - 2, floor - band, 1, INK);
    dessen::dsn_stroke(left, top, width, height, INK, 2, radius, corners);
}

/// An agave: a burst of thin, stiff leaves.
fn agave(root: Point, width: i32, height: i32) {
    let half = (width / 12).max(2);
    // Tips in twentieths of the width across and the height up, from the
    // lowest leaves in, so the upright ones lie over the others.
    let tips = [(10, 3), (9, 7), (7, 10), (5, 12), (2, 14)];
    for (across, up) in tips {
        leaf(root, -width * across / 20, height * up / 20, half);
        leaf(root, width * (across - 1) / 20, height * (up + 1) / 20, half);
    }
    leaf(root, 0, height * 12 / 20, half);
}

/// A snake plant: a few broad blades standing nearly straight.
fn blades(root: Point, width: i32, height: i32) {
    let half = (width / 8).max(2);
    leaf(root, -width * 3 / 10, height * 2 / 5, half);
    leaf(root, width * 3 / 10, height * 7 / 20, half);
    leaf(root, width / 7, height / 2, half);
    leaf(root, -width / 10, height * 11 / 20, half);
}

/// A cactus: three ribbed columns, each with knobs down its sides and pale
/// spines down its middle.
fn cactus(root: Point, width: i32, height: i32) {
    let wide = (width / 5).max(5);
    let knob = 1;
    // The outer columns, then the tall one between and in front of them.
    for (across, tall) in [(-wide, height * 9 / 20), (wide, height * 7 / 20), (0, height * 23 / 40)] {
        let (middle, top) = (root.x + across, root.y - tall);
        let corners = dessen::DSN_CORNERS_ALL;
        dessen::dsn_fill_round(middle - wide / 2 - 1, top - 1, wide + 2, tall + 1, PAPER, wide / 2, corners);
        dessen::dsn_fill_round(middle - wide / 2, top, wide, tall, INK, wide / 2, corners);
        let mut down = top + wide / 2;
        while down < root.y - knob {
            disc(middle - wide / 2, down, knob, INK);
            disc(middle + wide / 2, down, knob, INK);
            Renderer::fill_rect(Rect::new(middle, down, knob, knob), false);
            down += 4;
        }
    }
}

/// An echeveria: a rosette of round leaves spilling over the pot's rim.
fn rosette(root: Point, width: i32) {
    let leaf = (width / 6).max(3);
    // Leaf centres in tenths of a leaf's radius, back to front.
    let leaves = [
        (0, -22),
        (-13, -16),
        (13, -16),
        (-23, -6),
        (23, -6),
        (-7, -10),
        (7, -10),
        (-14, 1),
        (14, 1),
        (0, -1),
    ];
    for (across, down) in leaves {
        let (x, y) = (root.x + leaf * across / 10, root.y + leaf * down / 10);
        disc(x, y, leaf + 1, PAPER);
        disc(x, y, leaf, INK);
    }
    // The bud at the heart of it.
    let heart = root.y - leaf * 11 / 10;
    disc(root.x, heart, leaf * 2 / 3 + 1, PAPER);
    disc(root.x, heart, leaf * 2 / 3, INK);
}

/// The plant the player put in a cubby, in its pot: one of four, picked by
/// `kind`.
fn plant(centre: i32, floor: i32, width: i32, height: i32, kind: u32) {
    let tall = height * 2 / 5;
    pot(centre, floor, width * 7 / 10, tall);
    let root = Point::new(centre, floor - tall + 2);
    match kind % 4 {
        0 => agave(root, width, height),
        1 => cactus(root, width, height),
        2 => rosette(root, width),
        _ => blades(root, width, height),
    }
}

/// A tick, for something right.
pub fn tick(centre: Point, size: i32) {
    let (x, y) = (centre.x, centre.y);
    dessen::dsn_line(x - size, y, x - size / 3, y + size * 2 / 3, 3, INK);
    dessen::dsn_line(x - size / 3, y + size * 2 / 3, x + size, y - size * 2 / 3, 3, INK);
}

/// A cross, for something not allowed.
pub fn cross(centre: Point, size: i32) {
    let (x, y) = (centre.x, centre.y);
    dessen::dsn_line(x - size, y - size, x + size, y + size, 3, INK);
    dessen::dsn_line(x + size, y - size, x - size, y + size, 3, INK);
}

/// A warning badge: an ink disc with a pale exclamation mark.
fn warning(across: i32, down: i32, radius: i32) {
    disc(across, down, radius + 1, PAPER);
    disc(across, down, radius, INK);
    let stroke = (radius / 3).max(2);
    let left = across - stroke / 2;
    Renderer::fill_rect(Rect::new(left, down - radius * 3 / 5, stroke, radius * 3 / 4), false);
    Renderer::fill_rect(Rect::new(left, down + radius * 2 / 5, stroke, stroke), false);
}

/// One cubby: its uprights, the shelf under it, the shadow of the shelf over
/// it, and what it holds. `seed` picks its books or which plant it is, and
/// `crowded` badges a plant that touches another.
pub fn cubby(cell: Rect, content: Content, seed: u32, crowded: bool) {
    let (left, top, side) = (cell.origin.x, cell.origin.y, cell.size.width);
    let shelf = (side / 10).max(4);
    let floor = top + side - shelf;
    let shadow = (side / 14).max(2);

    Renderer::fill_rect(Rect::new(left, top, 1, side), true);
    Renderer::fill_rect(Rect::new(left + side - 1, top, 1, side), true);
    Renderer::fill_rect_dither(Rect::new(left + 1, top, side - 2, shadow), true);
    Renderer::fill_rect_dither(Rect::new(left + 1, top, shadow / 2, side - shelf), true);
    plank(Rect::new(left, floor, side, shelf), false);

    let (inner_left, inner_right, inner_top) = (left + 4, left + side - 4, top + shadow + 3);
    match content {
        Content::Empty => {}
        Content::Book => books(inner_left, inner_right, inner_top, floor, seed),
        Content::Plant => {
            let height = floor - inner_top;
            // Stirred first: neighbouring cubbies' seeds are too alike.
            let mut seed = seed;
            next(&mut seed);
            let kind = next(&mut seed) as u32;
            plant(left + side / 2, floor, side * 4 / 5, height, kind);
            if crowded {
                let radius = (side / 9).max(5);
                warning(left + side - radius - 4, top + shadow + radius + 3, radius);
            }
        }
        Content::Mark => {
            let arm = (side / 10).max(3);
            let (across, down) = (left + side / 2, top + (floor - top) / 2);
            dessen::dsn_line(across - arm, down - arm, across + arm, down + arm, 2, INK);
            dessen::dsn_line(across + arm, down - arm, across - arm, down + arm, 2, INK);
        }
    }
}
