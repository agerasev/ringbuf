mod atomic;
mod local;

pub use atomic::*;
pub use local::*;

use core::num::NonZeroUsize;

/// Ring buffer counter. Contains `head` and `tail` positions and also cached capacity (`len`).
///
/// When an item is extracted from the ring buffer it is taken from the **head** side. New items are appended to the **tail** side.
///
/// The valid values for `head` and `tail` are allowed be less than `2 * len` (instead of `len`).
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head == len` modulo `2 * len`) without using an extra space in container.
pub trait Counter: Sized {
    /// Acquire the local **head** counter.
    /// See `LocalHeadCounter` for more information.
    ///
    /// # Safety
    ///
    /// This function is allowed to call only if there is no another alive `LocalHeadCounter` for this counter.
    unsafe fn acquire_head(&self) -> LocalHeadCounter<'_, Self> {
        LocalHeadCounter::new(self)
    }

    /// Acquire the local **tail** counter.
    /// See `LocalTailCounter` for more information.
    ///
    /// # Safety
    ///
    /// This function is allowed to call only if there is no another alive `LocalTailCounter` for this counter.
    unsafe fn acquire_tail(&self) -> LocalTailCounter<'_, Self> {
        LocalTailCounter::new(self)
    }
}

/// Counter that may be created with default state.
pub trait DefaultCounter: Counter {
    /// Creates an empty counter of `len` capacity.
    fn with_capacity(len: NonZeroUsize) -> Self;
}
