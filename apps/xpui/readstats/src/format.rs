//! How times and counts read on the cards.

use alloc::string::String;

/// "45m", "5h 45m", "123h".
pub fn duration(seconds: u32) -> String {
    let (hours, minutes) = (seconds / 3600, seconds / 60 % 60);
    match hours {
        0 => kit::join!(minutes, "m"),
        1..=99 => kit::join!(hours, "h ", minutes, "m"),
        _ => kit::join!(hours, "h"),
    }
}

/// "1 day", "12 days".
pub fn days(count: u32) -> String {
    let unit = if count == 1 { "day" } else { "days" };
    kit::join!(count, " ", unit)
}
