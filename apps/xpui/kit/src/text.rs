//! Text in a box: the firmware's faces, cut to fit and aligned.

use alloc::string::String;
use xpui::Point;
use xpui::Renderer;
use xpui::host::{FontId, FontStyle};

/// One of the firmware's three faces, regular or bold.
///
/// xpui's fonts cover the interface sizes; the title face, which the stats
/// cards use for their values, is only dessen's, so this names all three.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Face {
    font: i32,
    bold: bool,
}

impl Face {
    /// Captions and secondary labels.
    pub const SMALL: Face = Face::new(dessen::DSN_FONT_SMALL);
    /// Body text.
    pub const BODY: Face = Face::new(dessen::DSN_FONT_BODY);
    /// Headings and big values.
    pub const TITLE: Face = Face::new(dessen::DSN_FONT_TITLE);

    const fn new(font: i32) -> Face {
        Face { font, bold: false }
    }

    /// The same face in bold.
    pub const fn bold(self) -> Face {
        Face { bold: true, ..self }
    }

    /// The face in bold when `on`.
    pub const fn bold_if(self, on: bool) -> Face {
        Face { bold: on, ..self }
    }

    fn flags(self) -> i32 {
        if self.bold { dessen::DSN_TEXT_BOLD } else { 0 }
    }

    /// The height of one line, which a text box must have at least.
    pub fn line_height(self) -> i32 {
        dessen::dsn_line_height(self.font)
    }

    /// How wide `text` draws.
    pub fn width(self, text: &str) -> i32 {
        (dessen::dsn_measure_text(text.as_bytes(), self.font, self.flags()) >> 16) as i32
    }

    /// Paints `text` with its top-left corner at `origin`.
    pub fn draw(self, origin: Point, text: &str) {
        let style = if self.bold {
            FontStyle::Bold
        } else {
            FontStyle::Regular
        };
        // xpui's font ids are dessen's font numbers plus one; see xpui-dessen.
        Renderer::draw_text(origin, text, FontId(self.font + 1), style);
    }
}

/// Where a line of text sits across its box.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// `text`, cut with an ellipsis to fit `width` when it is wider.
pub fn fit(text: &str, face: Face, width: i32) -> String {
    if face.width(text) <= width {
        return String::from(text);
    }
    let mut end = text.len();
    while end > 0 {
        end -= 1;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let cut = crate::join!(text[..end].trim_end(), "...");
        if face.width(&cut) <= width {
            return cut;
        }
    }
    String::new()
}

/// Paints one line of `text` in the box `width` wide from (`left`, `top`),
/// cut to fit and aligned across it.
pub fn text(left: i32, top: i32, width: i32, text: &str, face: Face, align: Align) {
    if width <= 0 || text.is_empty() {
        return;
    }
    let line = fit(text, face, width);
    let start = match align {
        Align::Left => left,
        Align::Center => left + (width - face.width(&line)) / 2,
        Align::Right => left + width - face.width(&line),
    };
    face.draw(Point::new(start, top), &line);
}

/// Paints `text` over at most `lines` lines, broken at spaces and centred,
/// from `top`; the last line is cut to fit. Returns the lines used.
pub fn paragraph(left: i32, top: i32, width: i32, text: &str, face: Face, lines: i32) -> i32 {
    let mut words = text.split(' ').peekable();
    let mut row = 0;
    while words.peek().is_some() && row < lines {
        let last_row = row + 1 == lines;
        let mut line = String::new();
        while let Some(word) = words.peek() {
            let longer = if line.is_empty() {
                String::from(*word)
            } else {
                crate::join!(line, " ", *word)
            };
            // The last row takes every remaining word, and `text` cuts it.
            if !line.is_empty() && !last_row && face.width(&longer) > width {
                break;
            }
            line = longer;
            words.next();
        }
        let top = top + row * face.line_height();
        self::text(left, top, width, &line, face, Align::Center);
        row += 1;
    }
    row
}
