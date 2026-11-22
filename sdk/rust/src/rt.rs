// The Rust counterpart of sdk/crt0.S: the 16-byte image header at address 0,
// then `_start`, which calls the app's `main` and exits with its result. The
// header values come from sdk/dessen_abi.h through build.rs; `__mem_size` from
// the link script.

use crate::{DSN_API_VERSION, DSN_MAGIC, DSN_SYS_EXIT, dsn_exit};

core::arch::global_asm!(
    ".section .header, \"a\"",
    ".word {magic}",
    ".word __mem_size",
    ".word _start",
    ".word {version}",
    ".section .text._start, \"ax\"",
    ".globl _start",
    "_start:", // the host sets sp to the top of guest memory
    "    call main",
    "    li a7, {exit}", // exit(main's return value)
    "    ecall",
    magic = const DSN_MAGIC,
    version = const DSN_API_VERSION,
    exit = const DSN_SYS_EXIT,
);

/// The exit code an app ends with when it panics.
///
/// The desktop runner prints it, so a panic is told apart from a fault (101,
/// 102) and from the app's own codes. The panic message is not printed: the
/// VM has no console, and formatting it would pull in `core::fmt`.
pub const PANIC_EXIT_CODE: i32 = 0x70;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    dsn_exit(PANIC_EXIT_CODE);
    loop {} // exit never returns; the loop only satisfies `!`
}

/// Sets the stack the app asks for, in bytes, where [`heap!`](crate::heap!)
/// sets its heap: `dessen::stack!(4 * 1024)`.
///
/// The image header then asks for code, data and heap plus this much stack,
/// and no more. A stack that runs out writes over the heap below it, with no
/// fault to say so, so size it from the deepest measured use with room to
/// spare. Without it the link's `APP_STACK` (or all of `APP_MEM`) is used.
#[macro_export]
macro_rules! stack {
    ($bytes:expr) => {
        // An absolute symbol the link script reads: see sdk/app.ld.
        core::arch::global_asm!(
            ".globl __app_stack",
            ".set __app_stack, {bytes}",
            bytes = const $bytes,
        );
    };
}

/// Makes `run` the app's entry point: `dessen::main!(run);`, with
/// `fn run() -> i32`. What it returns is the app's exit code.
///
/// The image's `_start` calls a function named `main`; this writes that
/// function, so an app needs no `#[unsafe(no_mangle)]` and no `extern "C"` of
/// its own, and can `#![forbid(unsafe_code)]`.
#[macro_export]
macro_rules! main {
    ($run:path) => {
        // `_start` (sdk/rust/src/rt.rs) calls `main` by name.
        #[unsafe(no_mangle)]
        extern "C" fn main() -> i32 {
            $run()
        }
    };
}
