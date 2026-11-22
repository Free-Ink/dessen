//! A `static` an app takes `&mut` to once: big state kept out of the heap and
//! off the stack, without `unsafe` in the app.

use core::cell::{Cell, UnsafeCell};

/// A value built at compile time, in the image's zeroed `.bss` or its data,
/// that [`take`](ConstStaticCell::take) hands out as `&'static mut` exactly
/// once.
///
/// For state too big for the stack and with no reason to be on the heap: a
/// table of counters, a scratch buffer. It costs its size in the image's
/// memory and nothing else. The name and the shape are those of the
/// `static_cell` crate's type of the same name.
///
/// ```ignore
/// static STATS: dessen::ConstStaticCell<Stats> = dessen::ConstStaticCell::new(Stats::ZERO);
///
/// let stats: &'static mut Stats = STATS.take().unwrap();
/// ```
pub struct ConstStaticCell<T> {
    taken: Cell<bool>,
    value: UnsafeCell<T>,
}

// Safety: a dessen app is one thread with no interrupts, so `take` cannot race
// itself, and the one `&mut` it hands out is the only way to the value. Only
// on the VM's target: a host test is multi-threaded, and there the cell is not
// `Sync`, so it cannot be put in a `static` to be shared.
#[cfg(target_arch = "riscv32")]
unsafe impl<T> Sync for ConstStaticCell<T> {}

impl<T> ConstStaticCell<T> {
    /// A cell holding `value`, for a `static`'s initialiser.
    pub const fn new(value: T) -> ConstStaticCell<T> {
        ConstStaticCell {
            taken: Cell::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// The value, the first time; `None` every time after.
    pub fn take(&'static self) -> Option<&'static mut T> {
        if self.taken.replace(true) {
            return None;
        }
        // Safety: `taken` was false, so no `&mut` to the value exists, and it is
        // now true, so none will be made again: this one is unique for 'static.
        Some(unsafe { &mut *self.value.get() })
    }
}

#[cfg(test)]
mod tests {
    use super::ConstStaticCell;
    use std::boxed::Box;

    #[test]
    fn hands_out_its_value_once() {
        let cell: &'static ConstStaticCell<[u32; 4]> =
            Box::leak(Box::new(ConstStaticCell::new([0; 4])));
        let value = cell.take().expect("the first take");
        value[2] = 7;
        assert!(cell.take().is_none(), "a second take would alias the first");
        assert_eq!(value[2], 7);
    }
}
