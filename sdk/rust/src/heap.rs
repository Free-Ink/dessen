//! A small allocator for dessen apps, over a static array.
//!
//! The VM gives an app one fixed block of memory and no `malloc`. An app that
//! wants `alloc` (`Vec`, `Box`, `String`, or a framework built on them) sets
//! aside part of that block as a heap:
//!
//! ```ignore
//! dessen::heap!(24 * 1024); // `static HEAP`, installed as the global allocator
//! ```
//!
//! The heap counts against the app's memory (64 KB unless it links with
//! `--defsym=APP_MEM=…`, or just what it uses with `--defsym=APP_STACK=…`),
//! next to its code, data and stack.
//!
//! # How it works
//!
//! First fit over a free list kept in address order, with no header on a live
//! allocation: `dealloc` is told the size, so nothing needs storing. Every size
//! and every block boundary is a multiple of the list node's size, so any piece
//! left over by a split can always hold a node, and no byte is ever lost. Freeing
//! merges with both neighbours, so a heap whose allocations are all freed is
//! one block again ([`Heap::largest_free`] equals [`Heap::capacity`]).
//!
//! The VM runs one app on one thread with no interrupts, so there is no
//! locking. That is also why [`Heap`] is `Sync` despite holding plain cells: it
//! must not be shared with another thread, and on dessen there is none.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self, null_mut};

/// A free block: its size in bytes (a multiple of [`GRAN`]) and the next free
/// block, at a higher address.
#[repr(C)]
struct Node {
    size: usize,
    next: *mut Node,
}

/// The allocation granule: every size and boundary is a multiple of this.
const GRAN: usize = core::mem::size_of::<Node>();

/// The backing array, aligned so its first byte is a valid block boundary.
#[repr(C, align(16))]
struct Arena<const N: usize>([u8; N]);

/// The free list's head. Kept apart from the arena: a `&mut` to the state must
/// not cover the bytes an app's live allocations point into, so the arena is
/// only ever reached through a raw pointer.
struct State {
    head: *mut Node,
    ready: bool,
}

/// An `N`-byte heap. Use it through [`heap!`](crate::heap!) as the global
/// allocator, or call [`GlobalAlloc`] on it directly.
pub struct Heap<const N: usize> {
    arena: UnsafeCell<Arena<N>>,
    state: UnsafeCell<State>,
}

// Safety: see the module docs. A dessen app is one thread with no interrupts,
// so no two calls into the allocator can overlap.
unsafe impl<const N: usize> Sync for Heap<N> {}

impl<const N: usize> Heap<N> {
    /// An empty heap. The free list is built on first use, since a `const fn`
    /// cannot take the address of the array it is building.
    pub const fn new() -> Self {
        Heap {
            arena: UnsafeCell::new(Arena([0; N])),
            state: UnsafeCell::new(State {
                head: null_mut(),
                ready: false,
            }),
        }
    }

    /// Bytes the heap can hand out in total: `N` rounded down to the granule.
    pub const fn capacity(&self) -> usize {
        N / GRAN * GRAN
    }

    /// Bytes currently free, across all blocks.
    pub fn free_bytes(&self) -> usize {
        self.fold(0, |sum, size| sum + size)
    }

    /// The largest single block free. An allocation bigger than this fails
    /// even when [`free_bytes`](Heap::free_bytes) is larger: that gap is
    /// fragmentation.
    pub fn largest_free(&self) -> usize {
        self.fold(0, |max, size| max.max(size))
    }

    /// The free-list state, built on first use.
    ///
    /// # Safety
    ///
    /// Single-threaded (module docs): no other `&mut` to the state is alive.
    #[allow(clippy::mut_from_ref)]
    unsafe fn state(&self) -> &mut State {
        // Safety: the caller's.
        let state = unsafe { &mut *self.state.get() };
        if !state.ready {
            state.ready = true;
            let size = N / GRAN * GRAN;
            if size > 0 {
                let node = self.arena.get().cast::<Node>();
                // Safety: the arena is 16-aligned and at least one granule long,
                // and nothing is allocated from it yet.
                unsafe {
                    node.write(Node {
                        size,
                        next: null_mut(),
                    })
                };
                state.head = node;
            }
        }
        state
    }

    fn fold(&self, init: usize, f: impl Fn(usize, usize) -> usize) -> usize {
        // Safety: single-threaded (module docs); nothing else holds the state.
        let state = unsafe { self.state() };
        let (mut acc, mut cur) = (init, state.head);
        while !cur.is_null() {
            // Safety: every node on the list lies inside the arena, written by us.
            unsafe {
                acc = f(acc, (*cur).size);
                cur = (*cur).next;
            }
        }
        acc
    }
}

impl<const N: usize> Heap<N> {
    /// Extends the live block at `ptr` from `old` to `new` bytes (both whole
    /// granules) by taking the front of the free block that starts where it
    /// ends. False, and nothing changed, when there is none or it is too small.
    ///
    /// # Safety
    ///
    /// `ptr` must be a live block of `old` bytes from this heap.
    unsafe fn grow_in_place(&self, ptr: *mut u8, old: usize, new: usize) -> bool {
        // Safety: single-threaded (module docs).
        let state = unsafe { self.state() };
        let end = ptr as usize + old;
        let mut prev: *mut Node = null_mut();
        let mut cur = state.head;
        while !cur.is_null() && (cur as usize) < end {
            prev = cur;
            // Safety: a live node.
            cur = unsafe { (*cur).next };
        }
        let need = new - old;
        // Safety: `cur`, when not null, is a live node.
        if cur as usize != end || unsafe { (*cur).size } < need {
            return false;
        }
        // Safety: `cur` is a live node; what is left of it stays free.
        unsafe {
            let (rest, next) = ((*cur).size - need, (*cur).next);
            let mut after = next;
            if rest > 0 {
                let tail = (end + need) as *mut Node;
                tail.write(Node { size: rest, next });
                after = tail;
            }
            if prev.is_null() {
                state.head = after;
            } else {
                (*prev).next = after;
            }
        }
        true
    }
}

impl<const N: usize> Default for Heap<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// `value` rounded up to a multiple of `multiple` (a power of two).
const fn round_up(value: usize, multiple: usize) -> usize {
    (value + multiple - 1) & !(multiple - 1)
}

/// The size a layout occupies: never zero, always whole granules.
fn block_size(layout: Layout) -> usize {
    round_up(layout.size().max(1), GRAN)
}

unsafe impl<const N: usize> GlobalAlloc for Heap<N> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Safety: single-threaded (module docs).
        let state = unsafe { self.state() };
        let size = block_size(layout);
        let align = layout.align().max(GRAN);

        let mut prev: *mut Node = null_mut();
        let mut cur = state.head;
        while !cur.is_null() {
            // Safety: list nodes are inside the arena and hold valid fields.
            let (base, have, next) = unsafe { (cur as usize, (*cur).size, (*cur).next) };
            let start = round_up(base, align);
            let end = start.checked_add(size);
            if let Some(end) = end.filter(|&end| end <= base + have) {
                let front = start - base; // a multiple of GRAN: both are aligned to it
                let back = base + have - end;
                // What follows `cur` once the allocation is cut out of it.
                let mut after = next;
                if back > 0 {
                    let tail = end as *mut Node;
                    // Safety: [end, end + back) is free, inside `cur`, GRAN-aligned.
                    unsafe { tail.write(Node { size: back, next }) };
                    after = tail;
                }
                if front > 0 {
                    // Keep the front piece as `cur`, shortened; its link is unchanged.
                    // Safety: `cur` is a live node.
                    unsafe {
                        (*cur).size = front;
                        (*cur).next = after;
                    }
                } else if prev.is_null() {
                    state.head = after;
                } else {
                    // Safety: `prev` is a live node.
                    unsafe { (*prev).next = after };
                }
                return start as *mut u8;
            }
            prev = cur;
            cur = next;
        }
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Safety: single-threaded (module docs).
        let state = unsafe { self.state() };
        let size = block_size(layout);
        let block = ptr.cast::<Node>();
        let addr = ptr as usize;

        // Find the free neighbours on either side, by address.
        let mut prev: *mut Node = null_mut();
        let mut next = state.head;
        while !next.is_null() && (next as usize) < addr {
            prev = next;
            // Safety: a live node.
            next = unsafe { (*next).next };
        }

        // Safety: `ptr` came from `alloc` with this layout, so the block is
        // GRAN-aligned, `size` long, and no longer in use.
        unsafe { block.write(Node { size, next }) };

        // Merge with the block after, if they touch.
        if !next.is_null() && addr + size == next as usize {
            // Safety: both are live nodes.
            unsafe {
                (*block).size += (*next).size;
                (*block).next = (*next).next;
            }
        }
        // Link in after `prev`, merging with it if they touch.
        if prev.is_null() {
            state.head = block;
        } else {
            // Safety: both are live nodes.
            unsafe {
                if prev as usize + (*prev).size == addr {
                    (*prev).size += (*block).size;
                    (*prev).next = (*block).next;
                } else {
                    (*prev).next = block;
                }
            }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Safety: `layout.align()` is valid; it came with `ptr`.
        let new = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        let (old_block, new_block) = (block_size(layout), block_size(new));
        // Same granule count: the block already fits, nothing to move.
        if new_block == old_block {
            return ptr;
        }
        // Shrinking frees the tail where it is, so a buffer cut down to what
        // it holds never needs room for a second copy.
        if new_block < old_block {
            // Safety: the tail is inside the live block, GRAN-aligned and whole
            // granules, and nothing uses it once the caller shrinks.
            unsafe {
                let tail = Layout::from_size_align_unchecked(old_block - new_block, GRAN);
                self.dealloc(ptr.add(new_block), tail);
            }
            return ptr;
        }
        // Growing into a free block right after it: no copy, and no second
        // block alive at once, which is what a growing Vec or String costs.
        // Safety: `ptr` is a live block of `old_block` bytes.
        if unsafe { self.grow_in_place(ptr, old_block, new_block) } {
            return ptr;
        }
        // Safety: the caller's contract for `realloc` covers `alloc`/`dealloc`.
        unsafe {
            let out = self.alloc(new);
            if !out.is_null() {
                ptr::copy_nonoverlapping(ptr, out, layout.size().min(new_size));
                self.dealloc(ptr, layout);
            }
            out
        }
    }
}

/// Installs an `$size`-byte [`Heap`] as the global allocator, as `static HEAP`
/// (or under the name given first: `heap!(ARENA, 8192)`).
#[macro_export]
macro_rules! heap {
    ($size:expr) => {
        $crate::heap!(HEAP, $size);
    };
    ($name:ident, $size:expr) => {
        #[global_allocator]
        static $name: $crate::heap::Heap<{ $size }> = $crate::heap::Heap::new();
    };
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::boxed::Box;
    use std::vec::Vec;

    fn heap<const N: usize>() -> &'static Heap<N> {
        Box::leak(Box::new(Heap::new()))
    }
    fn lay(size: usize, align: usize) -> Layout {
        Layout::from_size_align(size, align).unwrap()
    }

    #[test]
    fn starts_as_one_block() {
        let heap = heap::<4096>();
        assert_eq!(heap.free_bytes(), 4096);
        assert_eq!(heap.largest_free(), 4096);
    }

    #[test]
    fn alloc_and_free_restore_one_block() {
        let heap = heap::<4096>();
        let layouts = [lay(1, 1), lay(24, 8), lay(100, 4), lay(7, 2), lay(64, 16)];
        let blocks: Vec<_> = layouts
            .iter()
            .map(|&layout| unsafe { heap.alloc(layout) })
            .collect();
        assert!(blocks.iter().all(|block| !block.is_null()));
        for (block, layout) in blocks.iter().zip(layouts) {
            assert_eq!(*block as usize % layout.align(), 0);
        }
        // Free out of order: the middle first, so merging has to work both ways.
        for index in [2, 0, 4, 1, 3] {
            unsafe { heap.dealloc(blocks[index], layouts[index]) };
        }
        assert_eq!(heap.free_bytes(), 4096);
        assert_eq!(heap.largest_free(), 4096);
    }

    #[test]
    fn allocations_do_not_overlap() {
        let heap = heap::<4096>();
        let layout = lay(40, 8);
        let blocks: Vec<_> = (0..20).map(|_| unsafe { heap.alloc(layout) }).collect();
        for (index, block) in blocks.iter().enumerate() {
            unsafe { ptr::write_bytes(*block, index as u8, 40) };
        }
        for (index, block) in blocks.iter().enumerate() {
            let bytes = unsafe { core::slice::from_raw_parts(*block, 40) };
            assert!(bytes.iter().all(|&byte| byte == index as u8));
        }
    }

    #[test]
    fn exhaustion_returns_null_and_recovers() {
        let heap = heap::<256>();
        let big = lay(256, 8);
        let block = unsafe { heap.alloc(big) };
        assert!(!block.is_null());
        assert!(unsafe { heap.alloc(lay(1, 1)) }.is_null());
        unsafe { heap.dealloc(block, big) };
        assert_eq!(heap.largest_free(), 256);
    }

    #[test]
    fn large_alignment_keeps_the_padding_free() {
        let heap = heap::<4096>();
        let small = lay(8, 8);
        let first = unsafe { heap.alloc(small) }; // pushes the next free block off 256
        let wide = lay(32, 256);
        let aligned = unsafe { heap.alloc(wide) };
        assert_eq!(aligned as usize % 256, 0);
        // The gap between `first` and `aligned` stayed on the free list.
        let used = block_size(small) + block_size(wide);
        assert_eq!(heap.free_bytes(), 4096 - used);
        unsafe {
            heap.dealloc(first, small);
            heap.dealloc(aligned, wide);
        }
        assert_eq!(heap.largest_free(), 4096);
    }

    #[test]
    fn realloc_moves_and_keeps_contents() {
        let heap = heap::<4096>();
        let layout = lay(16, 8);
        let block = unsafe { heap.alloc(layout) };
        unsafe { ptr::copy_nonoverlapping(b"0123456789abcdef".as_ptr(), block, 16) };
        let blocker = unsafe { heap.alloc(lay(8, 8)) }; // forces the grow to move
        let grown = unsafe { heap.realloc(block, layout, 200) };
        assert_ne!(block, grown);
        assert_eq!(
            unsafe { core::slice::from_raw_parts(grown, 16) },
            b"0123456789abcdef"
        );
        unsafe {
            heap.dealloc(grown, lay(200, 8));
            heap.dealloc(blocker, lay(8, 8));
        }
        assert_eq!(heap.largest_free(), 4096);
    }

    #[test]
    fn realloc_grows_in_place_into_the_free_block_after() {
        let heap = heap::<4096>();
        let layout = lay(16, 8);
        let block = unsafe { heap.alloc(layout) };
        unsafe { ptr::copy_nonoverlapping(b"0123456789abcdef".as_ptr(), block, 16) };
        let grown = unsafe { heap.realloc(block, layout, 1000) };
        assert_eq!(block, grown);
        assert_eq!(
            unsafe { core::slice::from_raw_parts(grown, 16) },
            b"0123456789abcdef"
        );
        assert_eq!(heap.free_bytes(), 4096 - block_size(lay(1000, 8)));
        unsafe { heap.dealloc(grown, lay(1000, 8)) };
        assert_eq!(heap.largest_free(), 4096);
    }

    #[test]
    fn realloc_shrinks_in_place_and_frees_the_tail() {
        let heap = heap::<4096>();
        let big = lay(2000, 8);
        let block = unsafe { heap.alloc(big) };
        let after = unsafe { heap.alloc(lay(8, 8)) }; // the tail must not merge forward
        let shrunk = unsafe { heap.realloc(block, big, 100) };
        assert_eq!(block, shrunk);
        let used = block_size(lay(100, 8)) + block_size(lay(8, 8));
        assert_eq!(heap.free_bytes(), 4096 - used);
        unsafe {
            heap.dealloc(shrunk, lay(100, 8));
            heap.dealloc(after, lay(8, 8));
        }
        assert_eq!(heap.largest_free(), 4096);
    }
}
