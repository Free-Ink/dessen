// Self-check for the Rust SDK, run on the VM by `make rust-sdk`: the image
// header and entry point (it runs at all), the generated constants and calls,
// and the heap under `alloc` (growing and shrinking in place). Exits 0 on success, or the number of the first
// check that failed. A panic exits with dessen::PANIC_EXIT_CODE.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use dessen::*;

dessen::heap!(8 * 1024);

fn check(number: i32, ok: bool) -> Result<(), i32> {
    if ok { Ok(()) } else { Err(number) }
}

fn run() -> Result<(), i32> {
    // 1-2: constants come from dessen_abi.h, calls from abi/freeink.api.
    check(
        1,
        DSN_EV_CONFIRM == 1 && DSN_ERR_UNAVAILABLE == -4 && DSN_MAGIC == 0x314e_5344,
    )?;
    check(2, DSN_SHELF_RECORD_MAX == DSN_BOOK_PATH_MAX + 2 * 255 + 6)?;
    check(
        3,
        ev_type(DSN_EV_TAP as u32 | 7 << 8 | 9 << 20) == DSN_EV_TAP,
    )?;
    check(4, dsn_screen_width() > 0 && dsn_screen_height() > 0)?;

    // 5-8: the heap.
    let capacity = HEAP.capacity();
    check(5, HEAP.free_bytes() == capacity)?;
    {
        let mut values: Vec<u32> = Vec::new();
        for value in 0..500 {
            values.push(value * 3); // grows through several reallocs
        }
        let boxes: Vec<Box<[u8; 24]>> = (0..20u8).map(|fill| Box::new([fill; 24])).collect();
        let mut text = String::new();
        for _ in 0..10 {
            text.push_str("dessen ");
        }
        let tripled = |(index, &value): (usize, &u32)| value == index as u32 * 3;
        check(6, values.iter().enumerate().all(tripled))?;
        let filled =
            |(index, block): (usize, &[u8; 24])| block.iter().all(|&byte| byte == index as u8);
        let all_filled = boxes.iter().map(|block| &**block).enumerate().all(filled);
        check(7, all_filled && text.len() == 70)?;
    }
    // Everything dropped: the heap is one block again.
    check(
        8,
        HEAP.free_bytes() == capacity && HEAP.largest_free() == capacity,
    )?;

    // 9-10: realloc grows into the free space after a block and shrinks where
    // it is, so neither needs room for a second copy.
    let mut bytes: Vec<u8> = Vec::with_capacity(64);
    let start = bytes.as_ptr();
    bytes.reserve_exact(4096);
    check(9, bytes.as_ptr() == start)?;
    bytes.extend_from_slice(b"dessen");
    bytes.shrink_to_fit();
    let kept = capacity - HEAP.free_bytes();
    check(
        10,
        bytes.as_ptr() == start && kept <= 16 && bytes == b"dessen",
    )?;
    drop(bytes);
    Ok(())
}

fn dsn_screen_width() -> i32 {
    (dsn_screen() >> 16) as i32
}
fn dsn_screen_height() -> i32 {
    (dsn_screen() & 0xffff) as i32
}

dessen::main!(entry);

fn entry() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(number) => number,
    }
}
