use crate::utils::ring_buffer_ranges;
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Range, ptr};

/// Basic ring buffer functionality.
///
/// Provides an access to raw underlying memory and `head`/`tail` counters.
///
/// *It is recommended not to use this trait directly. Use [`Producer`](`crate::Producer`) and [`Consumer`](`crate::Consumer`) instead.*
///
/// # Details
///
/// The ring buffer consists of an array (of `capacity` size) and two counters: `head` and `tail`.
/// When an item is extracted from the ring buffer it is taken from the `head` position and after that `head` is incremented.
/// New item is appended to the `tail` position and `tail` is incremented after that.
///
/// The `head` and `tail` counters are modulo `2 * capacity` (not just `capacity`).
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head` modulo `2 * capacity` equals to `capacity`)
/// without using the space for an extra element in container.
/// And obviously we cannot store more than `capacity` items in the buffer, so `tail - head` modulo `2 * capacity` is not allowed to be greater than `capacity`.
pub trait RbBase<T> {
    /// Returns part of underlying raw ring buffer memory as slices.
    ///
    /// For more information see [`SharedStorage::as_mut_slices`](`crate::ring_buffer::storage::SharedStorage::as_mut_slices`).
    ///
    /// # Safety
    ///
    /// Only non-overlapping slices allowed to exist at the same time.
    ///
    /// Modifications of this data must properly update `head` and `tail` positions.
    ///
    /// *Accessing raw data is extremely unsafe.*
    /// It is recommended to use [`Consumer::as_slices`](`crate::Consumer::as_slices`) and [`Producer::free_space_as_slices`](`crate::Producer::free_space_as_slices`) instead.
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]);

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity_nonzero(&self) -> NonZeroUsize;

    /// Head position.
    fn head(&self) -> usize;

    /// Tail position.
    fn tail(&self) -> usize;

    /// Modulus for `head` and `tail` values.
    ///
    /// Equals to `2 * len`.
    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.capacity_nonzero().get()) }
    }

    /// The number of items stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.tail() - self.head()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (self.capacity_nonzero().get() + self.head() - self.tail()) % modulus
    }

    /// Checks if the occupied range is empty.
    fn is_empty(&self) -> bool {
        self.head() == self.tail()
    }

    /// Checks if the vacant range is empty.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}

/// Ring buffer read end.
///
/// Provides access to occupied memory and mechanism of item extraction.
///
/// *It is recommended not to use this trait directly. Use [`Producer`](`crate::Producer`) and [`Consumer`](`crate::Consumer`) instead.*
pub trait RbRead<T>: RbBase<T> {
    /// Sets the new **head** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::advance_head` instead.
    unsafe fn set_head(&self, value: usize);

    /// Move **head** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied area must be **initialized** before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of items in the ring buffer.*
    unsafe fn advance_head(&self, count: usize) {
        debug_assert!(count <= self.occupied_len());
        self.set_head((self.head() + count) % self.modulus());
    }

    /// Returns a pair of ranges of [`Self::occupied_slices`] location in underlying container.
    #[inline]
    fn occupied_ranges(&self) -> (Range<usize>, Range<usize>) {
        ring_buffer_ranges(self.capacity_nonzero(), self.head(), self.tail())
    }

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Returns a pair of slices of stored items, the second one may be empty.
    /// Elements with lower indices in slice are older. First slice contains older items that second one.
    ///
    /// # Safety
    ///
    /// All items are initialized. Elements must be removed starting from the beginning of first slice.
    /// When all items are removed from the first slice then items must be removed from the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance_head`] call with the number of items being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    #[inline]
    unsafe fn occupied_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.slices(self.head(), self.tail())
    }

    /// Removes items from the head of ring buffer and drops them.
    ///
    /// + If `count_or_all` is `Some(count)` then exactly `count` items will be removed.
    ///   *In debug mode panics if `count` is greater than number of items stored in the buffer.*
    /// + If `count_or_all` is `None` then all items in ring buffer will be removed.
    ///   *If there is concurring producer activity then the buffer may be not empty after this call.*
    ///
    /// Returns the number of removed items.
    ///
    /// # Safety
    ///
    /// Must not be called concurrently.
    unsafe fn skip_internal(&self, count_or_all: Option<usize>) -> usize {
        let (left, right) = self.occupied_slices();
        let count = match count_or_all {
            Some(count) => {
                debug_assert!(count <= left.len() + right.len());
                count
            }
            None => left.len() + right.len(),
        };
        for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
            ptr::drop_in_place(elem.as_mut_ptr());
        }
        self.advance_head(count);
        count
    }
}

/// Ring buffer write end.
///
/// Provides access to vacant memory and mechanism of item insertion.
///
/// *It is recommended not to use this trait directly. Use [`Producer`](`crate::Producer`) and [`Consumer`](`crate::Consumer`) instead.*
pub trait RbWrite<T>: RbBase<T> {
    /// Sets the new **tail** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::advance_tail` instead.
    unsafe fn set_tail(&self, value: usize);

    /// Move **tail** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in vacant area must be **de-initialized** (dropped) before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of vacant places in the ring buffer.*
    unsafe fn advance_tail(&self, count: usize) {
        debug_assert!(count <= self.vacant_len());
        self.set_tail((self.tail() + count) % self.modulus());
    }

    /// Returns a pair of ranges of [`Self::vacant_slices`] location in underlying container.
    #[inline]
    fn vacant_ranges(&self) -> (Range<usize>, Range<usize>) {
        ring_buffer_ranges(
            self.capacity_nonzero(),
            self.tail(),
            self.head() + self.capacity_nonzero().get(),
        )
    }

    /// Provides a direct access to the ring buffer vacant memory.
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    ///
    /// # Safety
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance_tail`] call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    #[inline]
    unsafe fn vacant_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.slices(self.tail(), self.head() + self.capacity_nonzero().get())
    }
}
