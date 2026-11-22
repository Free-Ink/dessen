// A dessen app written in Rust — proof the RV32IM runtime is language-agnostic.
// The ecall wrappers (dsn_clear, dsn_text_box, …) are generated from
// abi/freeink.api by tools/gen_api.py, same as the C header. This file links
// against the same sdk/crt0.S + sdk/app.ld as the C apps. no_std, no allocator.
//
//   rustc --target riscv32im-unknown-none-elf --crate-type staticlib \
//         -C panic=abort -C opt-level=s -o build/libhello_rs.a apps/hello.rs
//   (then link with gcc: see `make rust`)
#![no_std]
#![allow(dead_code)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    // Exit so the host leaves the app, instead of spinning until a force quit.
    dsn_exit(99);
    loop {}
}

// The generated ecall wrappers + syscall numbers.
include!("../sdk/dessen.rs");

fn text(x: i32, y: i32, s: &[u8]) {
    dsn_text_box(x, y, 0, 0, s, 0, 0, 1, 0); // w=h=0, FONT_SMALL, ALIGN_LEFT, 1 line
}

// crt0.S calls main, then exit(a7)'s with main's return value as the exit code.
#[no_mangle]
pub extern "C" fn main() -> i32 {
    dsn_clear();
    text(40, 60, b"Hello from Rust on dessen");
    text(40, 100, b"Press any button");
    dsn_refresh();
    dsn_wait_event() as i32 // exit code = the button pressed
}
