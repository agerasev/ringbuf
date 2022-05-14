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
    /// Capacity of the ring buffer.
    fn len(&self) -> NonZeroUsize;
    /// Head position.
    fn head(&self) -> usize;
    /// Tail position.
    fn tail(&self) -> usize;

    /// Sets the new **head** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recomended to use `Consumer::advance()` instead.
    unsafe fn set_head(&self, value: usize);
    /// Sets the new **tail** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recomended to use `Producer::advance()` instead.
    unsafe fn set_tail(&self, value: usize);

    #[inline]
    /// Modulus for `head` and `tail` values.
    ///
    /// Equals to `2 * len`.
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.len().get()) }
    }

    /// The number of items stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.tail() - self.head()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.head() - self.tail() - self.len().get()) % modulus
    }

    /// Checks if the occupied range is empty.
    fn is_empty(&self) -> bool {
        self.head() == self.tail()
    }

    /// Checks if the vacant range is empty.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }

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
