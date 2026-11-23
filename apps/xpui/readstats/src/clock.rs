//! Today's date and the time zone, from the board's clock.

/// What the clock says, as the stats need it.
#[derive(Clone, Copy)]
pub struct Clock {
    /// Whether the clock is set; without it "today" is the newest session.
    pub valid: bool,
    /// Today as a local day number (days since 1970-01-01); 0 when not valid.
    pub today: u32,
    /// Minutes behind UTC, as JavaScript's `getTimezoneOffset` gives them.
    pub offset_minutes: i32,
}

impl Clock {
    pub fn read() -> Clock {
        // dsn_clock_t: u16 year, then month, day, hour, minute, second bytes,
        // in local time.
        let mut clock = [0u8; 8];
        let year = if dessen::dsn_clock(&mut clock) >= 0 {
            u32::from(u16::from_le_bytes([clock[0], clock[1]]))
        } else {
            0
        };
        let [_, _, month, day, hour, minute, second, _] = clock;
        let valid = year >= 2000 && (1..=12).contains(&month) && (1..=31).contains(&day);
        let today = if valid {
            days_from_civil(year, u32::from(month), u32::from(day))
        } else {
            0
        };
        // The local clock against Unix time, to the quarter hour.
        let utc = i64::from(dessen::dsn_time());
        let local = i64::from(today) * 86400
            + i64::from(hour) * 3600
            + i64::from(minute) * 60
            + i64::from(second);
        // The difference is hours, not years: clamped first, it is divided in
        // 32 bits, with no 64-bit division routine linked for one line.
        let offset_minutes = if valid && utc > 0 {
            let seconds = (utc - local).clamp(-15 * 3600, 15 * 3600) as i32;
            (seconds / 900 * 15).clamp(-840, 720)
        } else {
            0
        };
        Clock {
            valid,
            today,
            offset_minutes,
        }
    }
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm), for a
/// year from 1970.
fn days_from_civil(year: u32, month: u32, day: u32) -> u32 {
    // Counted from March, so that the leap day ends the year.
    let year = if month <= 2 { year - 1 } else { year };
    let (era, year_of_era) = (year / 400, year % 400);
    let month_from_march = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    era * 146097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year - 719468
}
