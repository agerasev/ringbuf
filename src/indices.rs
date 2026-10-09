//! Cursor storage and synchronization policies.
use core::cell::Cell;
#[cfg(not(feature = "portable-atomic"))]
use core::sync::atomic::{AtomicUsize, Ordering};
use crossbeam_utils::CachePadded;
#[cfg(feature = "portable-atomic")]
use portable_atomic::{AtomicUsize, Ordering};

/// Stores all three cursors of a ring buffer.
///
/// # Safety
/// Loads must return the corresponding cursor, and setters must store it before
/// returning or unwinding. Implementations that are `Sync` must acquire published
/// writes/releases on loads and release preceding item accesses on stores.
/// No method may access the item storage or call user code.
pub unsafe trait Indices {
    fn new(released: usize, claimed: usize, published: usize) -> Self;
    fn read_released(&self) -> usize;
    fn read_claimed(&self) -> usize;
    fn write_published(&self) -> usize;
    /// # Safety
    /// The caller owns the consumer and has finished accessing the released slots.
    unsafe fn set_read_released(&self, value: usize);
    /// # Safety
    /// The caller owns the consumer and transfers ownership of this initialized range.
    unsafe fn set_read_claimed(&self, value: usize);
    /// # Safety
    /// The caller owns the producer and has initialized the published prefix.
    unsafe fn set_write_published(&self, value: usize);
}

/// Compact indices for use on one thread.
pub struct LocalIndices {
    released: Cell<usize>,
    claimed: Cell<usize>,
    published: Cell<usize>,
}

/// Atomic indices with the producer and consumer on separate cache lines.
pub struct AtomicIndices {
    read: CachePadded<(AtomicUsize, AtomicUsize)>,
    write: CachePadded<AtomicUsize>,
}

unsafe impl Indices for LocalIndices {
    fn new(released: usize, claimed: usize, published: usize) -> Self {
        Self {
            released: Cell::new(released),
            claimed: Cell::new(claimed),
            published: Cell::new(published),
        }
    }
    #[inline]
    fn read_released(&self) -> usize {
        self.released.get()
    }
    #[inline]
    fn read_claimed(&self) -> usize {
        self.claimed.get()
    }
    #[inline]
    fn write_published(&self) -> usize {
        self.published.get()
    }
    #[inline]
    unsafe fn set_read_released(&self, value: usize) {
        self.released.set(value);
    }
    #[inline]
    unsafe fn set_read_claimed(&self, value: usize) {
        self.claimed.set(value);
    }
    #[inline]
    unsafe fn set_write_published(&self, value: usize) {
        self.published.set(value);
    }
}

unsafe impl Indices for AtomicIndices {
    fn new(released: usize, claimed: usize, published: usize) -> Self {
        Self {
            read: CachePadded::new((AtomicUsize::new(released), AtomicUsize::new(claimed))),
            write: CachePadded::new(AtomicUsize::new(published)),
        }
    }
    #[inline]
    fn read_released(&self) -> usize {
        self.read.0.load(Ordering::Acquire)
    }
    #[inline]
    fn read_claimed(&self) -> usize {
        self.read.1.load(Ordering::Acquire)
    }
    #[inline]
    fn write_published(&self) -> usize {
        self.write.load(Ordering::Acquire)
    }
    #[inline]
    unsafe fn set_read_released(&self, value: usize) {
        self.read.0.store(value, Ordering::Release);
    }
    #[inline]
    unsafe fn set_read_claimed(&self, value: usize) {
        self.read.1.store(value, Ordering::Release);
    }
    #[inline]
    unsafe fn set_write_published(&self, value: usize) {
        self.write.store(value, Ordering::Release);
    }
}
