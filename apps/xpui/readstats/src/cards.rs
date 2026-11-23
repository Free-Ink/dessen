//! The stat cards, each measured or drawn by one function.

use alloc::string::String;
use kit::{Align, Face};
use xpui::{Point, Rect};

use crate::format::{days, duration};
use crate::layout::{GAP, GUTTER, Layout, PAD};
use crate::log::{DAYS, Stats};
use crate::paint::{bar, black, dark, heat, light};

/// The cards, in the order they flow down the pages.
#[derive(Clone, Copy)]
pub enum Card {
    /// Today, the last 7 days, the streak and the total.
    Summary,
    /// A heatmap of days, a column per week.
    Heatmap,
    /// Minutes per day over the last week.
    Week,
    /// Reading by the hour it started.
    TimeOfDay,
    /// The three books read longest.
    MostRead,
    /// Sessions, their average, the best streak and books' worth read.
    Totals,
}

impl Card {
    pub const ALL: [Card; 6] = [
        Card::Summary,
        Card::Heatmap,
        Card::Week,
        Card::TimeOfDay,
        Card::MostRead,
        Card::Totals,
    ];
}

/// Two letters per weekday, from Sunday.
const WEEKDAYS: &str = "SuMoTuWeThFrSa";

/// Today's weekday, 0 for Sunday: 1970-01-01, day 0, was a Thursday.
fn weekday(stats: &Stats) -> u32 {
    (stats.end + 4) % 7
}

impl Layout {
    /// Draws `card` at `top` when `draw`, and returns its height; 0 when it
    /// has nothing to show.
    pub fn card(&self, card: Card, stats: &Stats, origin: Point, top: i32, draw: bool) -> i32 {
        let frame = Rect::new(origin.x + GUTTER, top, self.content_width(), 0);
        match card {
            Card::Summary => self.summary(stats, origin, top, draw),
            Card::Heatmap => self.heatmap(stats, frame, draw),
            Card::Week => self.week(stats, frame, draw),
            Card::TimeOfDay => self.time_of_day(stats, frame, draw),
            Card::MostRead => self.most_read(stats, frame, draw),
            Card::Totals => self.totals(stats, origin, top, draw),
        }
    }

    /// A card's outline, `height` tall within `frame`'s width, with its label.
    fn outline(&self, frame: Rect, height: i32, label: &str) {
        let (left, top, width) = (frame.origin.x, frame.origin.y, frame.size.width);
        kit::frame(Rect::new(left, top, width, height), 1);
        kit::text(
            left + PAD,
            top + PAD / 2,
            width - 2 * PAD,
            label,
            Face::SMALL,
            Align::Left,
        );
    }

    /// Four label and value tiles: one row on a wide screen, 2 × 2 otherwise.
    fn tiles(&self, origin: Point, top: i32, draw: bool, tiles: [(&str, String); 4]) -> i32 {
        let width = self.content_width();
        let columns = if width >= 640 { 4 } else { 2 };
        let rows = 4 / columns;
        let tile_height = self.head() + self.title_line + PAD;
        if draw {
            let tile_width = (width - (columns - 1) * GAP) / columns;
            for (index, (label, value)) in tiles.iter().enumerate() {
                let (column, row) = (index as i32 % columns, index as i32 / columns);
                let left = origin.x + GUTTER + column * (tile_width + GAP);
                let tile = Rect::new(left, top + row * (tile_height + GAP), tile_width, 0);
                self.outline(tile, tile_height, label);
                let fits = Face::TITLE.bold().width(value) <= tile_width - 2 * PAD;
                let face = if fits { Face::TITLE } else { Face::BODY };
                let value_top = tile.origin.y + self.head();
                kit::text(
                    left + PAD,
                    value_top,
                    tile_width - 2 * PAD,
                    value,
                    face.bold(),
                    Align::Left,
                );
            }
        }
        rows * tile_height + (rows - 1) * GAP
    }

    fn summary(&self, stats: &Stats, origin: Point, top: i32, draw: bool) -> i32 {
        let (current, _) = stats.streaks();
        let week: u32 = stats.day[DAYS - 7..].iter().sum();
        let tiles = [
            ("Today", duration(stats.day[DAYS - 1])),
            ("Last 7 days", duration(week)),
            ("Streak", days(current)),
            ("Total time", duration(stats.total_seconds)),
        ];
        self.tiles(origin, top, draw, tiles)
    }

    fn heatmap(&self, stats: &Stats, frame: Rect, draw: bool) -> i32 {
        let (left, top, width) = (frame.origin.x, frame.origin.y, frame.size.width);
        let inner = width - 2 * PAD;
        let cell = self.small_line * 3 / 4;
        let pitch = cell + (cell / 4).max(2);
        let spacing = pitch - cell;
        let weeks = ((inner + spacing) / pitch).min(53);
        let height = self.head() + 7 * pitch - spacing + PAD;
        if !draw {
            return height;
        }
        self.outline(frame, height, &kit::join!("Last ", weeks, " weeks"));
        // The legend, right of the label: the four levels, lightest first.
        let legend_top = top + PAD / 2 + (self.small_line - cell) / 2;
        for level in 0..4 {
            let legend_left = left + width - PAD - (4 - level) * pitch + spacing;
            heat(legend_left, legend_top, cell, level as u32);
        }
        // A column per week, newest on the right, Sunday on top.
        let grid_left = left + (width - weeks * pitch + spacing) / 2;
        let grid_top = top + self.head();
        let today = weekday(stats) as i32;
        for week in 0..weeks {
            for day in 0..7 {
                let days_ago = (weeks - 1 - week) * 7 + today - day;
                if days_ago < 0 {
                    continue; // later this week
                }
                let level = match stats.day[DAYS - 1 - days_ago as usize] / 60 {
                    0 => 0,
                    1..=14 => 1,
                    15..=44 => 2,
                    _ => 3,
                };
                heat(
                    grid_left + week * pitch,
                    grid_top + day * pitch,
                    cell,
                    level,
                );
            }
        }
        height
    }

    fn week(&self, stats: &Stats, frame: Rect, draw: bool) -> i32 {
        let (left, top, width) = (frame.origin.x, frame.origin.y, frame.size.width);
        let small = self.small_line;
        let area = 4 * small;
        let height = self.head() + small + area + 4 + small + PAD;
        if !draw {
            return height;
        }
        self.outline(frame, height, "Last 7 days (minutes)");
        let inner = width - 2 * PAD;
        let week = &stats.day[DAYS - 7..];
        let max = week.iter().copied().max().unwrap_or(0).max(1);
        let column_width = inner / 7;
        let base = top + self.head() + small + area;
        for (index, &seconds) in week.iter().enumerate() {
            let column_left = left + PAD + index as i32 * column_width;
            let bar_height = bar(seconds, max, area);
            let bar_left = column_left + column_width / 5;
            black(
                bar_left,
                base - bar_height,
                column_width * 3 / 5,
                bar_height,
            );
            if seconds > 0 {
                let minutes = kit::join!(seconds / 60);
                let minutes_top = base - bar_height - small;
                kit::text(
                    column_left,
                    minutes_top,
                    column_width,
                    &minutes,
                    Face::SMALL,
                    Align::Center,
                );
            }
            let name = (weekday(stats) as usize + 1 + index) % 7 * 2;
            let day = &WEEKDAYS[name..name + 2];
            kit::text(
                column_left,
                base + 4,
                column_width,
                day,
                Face::SMALL,
                Align::Center,
            );
        }
        dark(left + PAD, base, inner, 1);
        height
    }

    fn time_of_day(&self, stats: &Stats, frame: Rect, draw: bool) -> i32 {
        let (left, top, width) = (frame.origin.x, frame.origin.y, frame.size.width);
        let small = self.small_line;
        let area = 3 * small;
        let height = self.head() + area + 4 + small + PAD;
        if !draw {
            return height;
        }
        self.outline(frame, height, "Time of day");
        let max = stats.hour.iter().copied().max().unwrap_or(0).max(1);
        let column_width = (width - 2 * PAD) / 24;
        let chart_left = left + (width - 24 * column_width) / 2;
        let base = top + self.head() + area;
        for (hour, &seconds) in stats.hour.iter().enumerate() {
            let column_left = chart_left + hour as i32 * column_width;
            let bar_height = bar(seconds, max, area);
            black(
                column_left + 1,
                base - bar_height,
                (column_width - 2).max(1),
                bar_height,
            );
            if hour % 6 == 0 {
                let label = kit::join!(hour, ":00");
                kit::text(
                    column_left,
                    base + 4,
                    6 * column_width,
                    &label,
                    Face::SMALL,
                    Align::Left,
                );
            }
        }
        dark(chart_left, base, 24 * column_width, 1);
        height
    }

    fn most_read(&self, stats: &Stats, frame: Rect, draw: bool) -> i32 {
        let (left, top, width) = (frame.origin.x, frame.origin.y, frame.size.width);
        let shown = stats.book_count.min(3);
        if shown == 0 {
            return 0;
        }
        let body = self.body_line;
        let row_height = body + 4 + 6 + 10;
        let height = self.head() + shown as i32 * row_height + PAD - 10;
        if !draw {
            return height;
        }
        self.outline(frame, height, "Most read");
        let inner = width - 2 * PAD;
        let mut longest = 0;
        for (rank, book) in stats.most_read(shown).enumerate() {
            if rank == 0 {
                longest = book.seconds;
            }
            let row_top = top + self.head() + rank as i32 * row_height;
            let time = duration(book.seconds);
            let time_width = Face::SMALL.width(&time) + 8;
            let title_width = inner - time_width;
            kit::text(
                left + PAD,
                row_top,
                title_width,
                book.title(),
                Face::BODY,
                Align::Left,
            );
            let time_left = left + width - PAD - time_width;
            kit::text(
                time_left,
                row_top,
                time_width,
                &time,
                Face::SMALL,
                Align::Right,
            );
            let share = book.seconds / 60 * inner as u32 / (longest / 60).max(1);
            light(left + PAD, row_top + body + 4, inner, 6);
            black(
                left + PAD,
                row_top + body + 4,
                (share as i32).clamp(2, inner),
                6,
            );
        }
        height
    }

    fn totals(&self, stats: &Stats, origin: Point, top: i32, draw: bool) -> i32 {
        let (_, best) = stats.streaks();
        let tenths = stats.progress / 1000; // 10000 basis points is one whole book
        let tiles = [
            ("Sessions", kit::join!(stats.sessions)),
            (
                "Average session",
                duration(stats.total_seconds / stats.sessions.max(1)),
            ),
            ("Best streak", days(best)),
            (
                "Books' worth read",
                kit::join!(tenths / 10, ".", tenths % 10),
            ),
        ];
        self.tiles(origin, top, draw, tiles)
    }
}
