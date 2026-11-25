//! A stand-in for the dessen SDK on the host, for unit tests of the apps'
//! logic. The constants are the real ones (`sdk`, re-exported); the calls
//! answer from a [`World`] each test sets up with [`with`], one per test
//! thread, so tests running side by side never see each other's files.

use std::cell::RefCell;
use std::collections::HashMap;

pub use sdk::*;

/// What the calls answer from.
pub struct World {
    /// Files by path; `None` for a board that cannot read files at all.
    pub files: Option<HashMap<Vec<u8>, Vec<u8>>>,
    /// `dsn_clock`'s eight bytes; `None` for a board without a clock.
    pub clock: Option<[u8; 8]>,
    /// `dsn_time`.
    pub time: u32,
    /// xorshift32 state behind `dsn_random`.
    pub seed: u32,
    /// Numbers `dsn_random` gives before it falls back to the seed.
    pub randoms: Vec<u32>,
    /// `dsn_recents`' answer; `None` for no recents (an error).
    pub recents: Option<Vec<u8>>,
    /// `dsn_recent_shelf`'s records, one per book; `None` for firmware without it.
    pub shelf: Option<Vec<Vec<u8>>>,
    /// `dsn_book_progress` by path; missing is unknown (`DSN_ERR_IO`).
    pub progress: HashMap<Vec<u8>, i32>,
    /// `dsn_battery`.
    pub battery: i32,
    /// What the app asked to open: "home 1", "app /x", "book /y".
    pub opened: Vec<String>,
}

impl Default for World {
    fn default() -> World {
        World {
            files: Some(HashMap::new()),
            clock: None,
            time: 0,
            seed: 0x9e37_79b9,
            randoms: Vec::new(),
            recents: None,
            shelf: None,
            progress: HashMap::new(),
            battery: -1,
            opened: Vec::new(),
        }
    }
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
}

/// Changes this test's world.
pub fn with<R>(f: impl FnOnce(&mut World) -> R) -> R {
    WORLD.with(|world| f(&mut world.borrow_mut()))
}

/// A fresh world for this test, changed by `f`.
pub fn reset(f: impl FnOnce(&mut World)) {
    with(|world| {
        *world = World::default();
        f(world);
    });
}

/// `dsn_clock`'s bytes for a local date and time.
pub fn clock_bytes(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> [u8; 8] {
    let [low, high] = year.to_le_bytes();
    [low, high, month, day, hour, minute, second, 0]
}

/// Copies `data` into `out` as a call that fills a buffer does: the length,
/// or `DSN_ERR_TOO_BIG` writing nothing.
fn give(data: &[u8], out: &mut [u8]) -> i32 {
    if data.len() > out.len() {
        return DSN_ERR_TOO_BIG;
    }
    out[..data.len()].copy_from_slice(data);
    data.len() as i32
}

pub fn dsn_fs_read_at(path: &[u8], offset: u32, data: &mut [u8]) -> i32 {
    with(|world| {
        let Some(files) = &world.files else { return DSN_ERR_UNAVAILABLE };
        let Some(file) = files.get(path) else { return DSN_ERR_IO };
        let start = (offset as usize).min(file.len());
        let end = (start + data.len()).min(file.len());
        data[..end - start].copy_from_slice(&file[start..end]);
        (end - start) as i32
    })
}

pub fn dsn_clock(t: &mut [u8]) -> i32 {
    with(|world| match world.clock {
        Some(clock) if t.len() >= 8 => {
            t[..8].copy_from_slice(&clock);
            0
        }
        Some(_) => DSN_ERR_TOO_BIG,
        None => DSN_ERR_UNAVAILABLE,
    })
}

pub fn dsn_time() -> u32 {
    with(|world| world.time)
}

pub fn dsn_random() -> u32 {
    with(|world| {
        if !world.randoms.is_empty() {
            return world.randoms.remove(0);
        }
        let mut s = world.seed;
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        world.seed = s;
        s
    })
}

pub fn dsn_recents(list: &mut [u8]) -> i32 {
    with(|world| match &world.recents {
        Some(recents) => give(recents, list),
        None => DSN_ERR_UNAVAILABLE,
    })
}

pub fn dsn_recent_shelf(first: u32, count: i32, list: &mut [u8]) -> i32 {
    with(|world| {
        let Some(shelf) = &world.shelf else { return DSN_ERR_UNAVAILABLE };
        if !(1..=DSN_SHELF_PAGE_MAX).contains(&count) {
            return DSN_ERR_INVALID;
        }
        let page: Vec<u8> = shelf
            .iter()
            .skip(first as usize)
            .take(count as usize)
            .flat_map(|record| record.iter().copied().chain([b'\n']))
            .collect();
        give(&page, list)
    })
}

pub fn dsn_book_progress(path: &[u8]) -> i32 {
    with(|world| world.progress.get(path).copied().unwrap_or(DSN_ERR_IO))
}

pub fn dsn_battery() -> i32 {
    with(|world| world.battery)
}

fn opened(what: &str, detail: &[u8]) -> i32 {
    with(|world| world.opened.push(format!("{what} {}", String::from_utf8_lossy(detail))));
    0
}

pub fn dsn_open_home(dest: i32) -> i32 {
    opened("home", dest.to_string().as_bytes())
}

pub fn dsn_open_app(path: &[u8]) -> i32 {
    opened("app", path)
}

pub fn dsn_open_book(path: &[u8]) -> i32 {
    opened("book", path)
}

/// The SDK's cell, for a `static` in an app's file. On the VM it hands its
/// value out once; here every test may build its own, so each `take` hands
/// out a fresh copy of the value it was made with.
pub struct ConstStaticCell<T> {
    value: T,
}

// Safety: the value is only ever read (copied), never written in place.
unsafe impl<T> Sync for ConstStaticCell<T> {}

impl<T> ConstStaticCell<T> {
    pub const fn new(value: T) -> ConstStaticCell<T> {
        ConstStaticCell { value }
    }

    /// A copy of the initial value, leaked for the rest of the test run.
    pub fn take(&'static self) -> Option<&'static mut T> {
        // Safety: the cells this stands in for hold plain data (counters,
        // buffers): nothing owned, nothing dropped, so a bitwise copy is a
        // true copy.
        Some(Box::leak(Box::new(unsafe { std::ptr::read(&self.value) })))
    }
}
