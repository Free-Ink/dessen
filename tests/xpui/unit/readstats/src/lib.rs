//! readstats' files that draw nothing: the log tally, the JSON reader, the
//! clock and the formats on its cards.
#![allow(dead_code)]

extern crate alloc;

#[path = "../../../../../apps/xpui/readstats/src/bytes.rs"]
mod bytes;
#[path = "../../../../../apps/xpui/readstats/src/clock.rs"]
mod clock;
#[path = "../../../../../apps/xpui/readstats/src/format.rs"]
mod format;
#[path = "../../../../../apps/xpui/readstats/src/json.rs"]
mod json;
#[path = "../../../../../apps/xpui/readstats/src/log.rs"]
mod log;

#[cfg(test)]
mod tests;
