//! The dessen app SDK for Rust.
//!
//! A dessen app is an RV32IM image run inside the reader's firmware by the
//! dessen VM. This crate is everything such an app links besides its own code:
//!
//! - the **calls** (`dsn_clear`, `dsn_text_box`, …), generated from
//!   `abi/freeink.api` exactly as the C header is;
//! - the **constants** from `sdk/dessen_abi.h` (`DSN_EV_*`, `DSN_FONT_*`, …);
//! - with the default `rt` feature, the **image header, entry point and panic
//!   handler** (what `crt0.S` is for a C app) and the link script;
//! - with the `heap` feature, an **allocator** over a static array, for apps
//!   that use `alloc` (see [`heap!`]).
//!
//! An app names its entry point with [`main!`], and needs no `unsafe` at all:
//!
//! ```ignore
//! #![no_std]
//! #![no_main]
//! #![forbid(unsafe_code)]
//! use dessen::*;
//!
//! dessen::main!(run);
//!
//! fn run() -> i32 {
//!     dsn_clear();
//!     dsn_text_box(40, 60, 0, 0, b"Hello", DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);
//!     dsn_refresh();
//!     dsn_wait_event() as i32
//! }
//! ```
//!
//! and builds with `--target riscv32im-unknown-none-elf` and
//! `-C link-arg=-Tdessen.ld` (add `-C link-arg=--defsym=APP_MEM=<bytes>` to ask
//! for more than the default 64 KB, and `--defsym=APP_STACK=<bytes>` to ask only
//! for code, data and heap plus that much stack, with `APP_MEM` as the ceiling;
//! or set the stack in the app with [`stack!`], next to its [`heap!`]).
//! Never `riscv32imc`: the VM does not decode compressed instructions.
#![cfg_attr(not(test), no_std)]

#[cfg(target_arch = "riscv32")]
mod sys {
    include!("../../dessen.rs");
}
#[cfg(target_arch = "riscv32")]
pub use sys::*;

mod abi {
    #![allow(unused_parens)]
    include!(concat!(env!("OUT_DIR"), "/abi.rs"));
}
pub use abi::*;

#[cfg(all(feature = "rt", target_arch = "riscv32"))]
mod rt;
#[cfg(all(feature = "rt", target_arch = "riscv32"))]
pub use rt::PANIC_EXIT_CODE;

#[cfg(feature = "heap")]
pub mod heap;

mod cell;
pub use cell::ConstStaticCell;

/// The event type: one of the `DSN_EV_*` constants (`DSN_EV_TYPE` in C).
pub const fn ev_type(event: u32) -> i32 {
    (event & 0xff) as i32
}

/// The x of a tap or long press, in safe-area coordinates (`DSN_EV_X` in C).
pub const fn ev_x(event: u32) -> i32 {
    ((event >> 8) & 0xfff) as i32
}

/// The y of a tap or long press, in safe-area coordinates (`DSN_EV_Y` in C).
pub const fn ev_y(event: u32) -> i32 {
    ((event >> 20) & 0xfff) as i32
}
