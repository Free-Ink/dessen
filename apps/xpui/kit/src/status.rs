//! A home screen's title line: the clock and the battery.

use alloc::string::String;
use alloc::vec::Vec;

/// "9:41 AM", from the board's clock; `None` when it has none.
pub fn clock() -> Option<String> {
    // dsn_clock_t: u16 year, then month, day, hour, minute, second bytes.
    let mut clock = [0u8; 8];
    // A year of zero is a clock that was never set.
    if dessen::dsn_clock(&mut clock) < 0 || clock[0] | clock[1] == 0 {
        return None;
    }
    let (hour, minute) = (u32::from(clock[4]), clock[5]);
    let twelve_hour = if hour % 12 == 0 { 12 } else { hour % 12 };
    let half = if hour < 12 { "AM" } else { "PM" };
    Some(crate::join!(
        twelve_hour,
        ":",
        crate::Padded(u32::from(minute), 2),
        " ",
        half
    ))
}

/// "87%", the battery's charge; `None` when the board does not say.
pub fn battery() -> Option<String> {
    // The percent is the low byte; the power state sits above it.
    let battery = dessen::dsn_battery();
    (battery >= 0).then(|| crate::join!(battery & 0xff, "%"))
}

/// "9:41 AM  87%", from whichever of the clock and battery the board has,
/// with `middle` between them when given.
pub fn status(middle: Option<&str>) -> String {
    let parts: Vec<String> = [clock(), middle.map(String::from), battery()]
        .into_iter()
        .flatten()
        .collect();
    if parts.is_empty() {
        String::from("Home")
    } else {
        parts.join("  ")
    }
}
