use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Range, ptr};

/// Returns a pair of ranges between `begin` and `end` positions in a ring buffer with specific `capacity`.
///
/// `begin` and `end` may be arbitrary large, but must satisfy the following condition: `0 <= (begin - end) % (2 * capacity) <= capacity`.
/// Actual positions are taken modulo `capacity`.
///
/// The first range starts from `begin`. If the first slice is empty then second slice is empty too.
pub fn ranges(capacity: NonZeroUsize, begin: usize, end: usize) -> (Range<usize>, Range<usize>) {
    let (head_quo, head_rem) = (begin / capacity, begin % capacity);
    let (tail_quo, tail_rem) = (end / capacity, end % capacity);

    if (head_quo + tail_quo) % 2 == 0 {
        (head_rem..tail_rem, 0..0)
    } else {
        (head_rem..capacity.get(), 0..tail_rem)
    }
}

pub trait RawStorage {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    /// Modulus for pointers to item in ring buffer storage.
    ///
    /// Equals to `2 * capacity`.
    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.capacity().get()) }
    }

    /// Get mutable slice of ring buffer data in specified `range`.
    ///
    /// # Safety
    ///
    /// `range` must be a subset of `0..capacity`.
    ///
    /// Slices with overlapping lifetimes must not overlap.
    #[allow(clippy::mut_from_ref)]
    unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<Self::Item>];

    /// Returns part of underlying raw ring buffer memory as slices.
    ///
    /// # Safety
    ///
    /// Only non-overlapping slices allowed to exist at the same time.
    #[inline]
    unsafe fn slices(
        &self,
        begin: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let (first, second) = ranges(Self::capacity(self), begin, end);
        (self.slice(first), self.slice(second))
    }
}

/// Basic ring buffer functionality.
///
/// Provides an access to raw underlying memory and `read`/`write` counters.
///
/// *It is recommended not to use this trait directly. Use [`Producer`](`crate::Producer`) and [`Consumer`](`crate::Consumer`) instead.*
///
/// # Details
///
/// The ring buffer consists of an array (of `capacity` size) and two counters: `read` and `write`.
/// When an item is extracted from the ring buffer it is taken from the `read` position and after that `read` is incremented.
/// New item is appended to the `write` position and `write` is incremented after that.
///
/// The `read` and `write` counters are modulo `2 * capacity` (not just `capacity`).
/// It allows us to distinguish situations when the buffer is empty (`read == write`) and when the buffer is full (`write - read` modulo `2 * capacity` equals to `capacity`)
/// without using the space for an extra element in container.
/// And obviously we cannot store more than `capacity` items in the buffer, so `write - read` modulo `2 * capacity` is not allowed to be greater than `capacity`.
pub trait RawRb: RawStorage {
    /// Read end position.
    fn read_end(&self) -> usize;

    /// Write ends position.
    fn write_end(&self) -> usize;

    /// The number of items stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.write_end() - self.read_end()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (self.capacity().get() + self.read_end() - self.write_end()) % modulus
    }

    /// Checks if the occupied range is empty.
    fn is_empty(&self) -> bool {
        self.read_end() == self.write_end()
    }

    /// Checks if the vacant range is empty.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }

    /// Sets the new **read** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_read_end` instead.
    unsafe fn set_read_end(&self, value: usize);

    /// Move **read** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied area must be **initialized** before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of items in the ring buffer.*
    unsafe fn move_read_end(&self, count: usize) {
        debug_assert!(count <= self.occupied_len());
        self.set_read_end((self.read_end() + count) % self.modulus());
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
    /// *This method must be followed by [`Self::move_read_end`] call with the number of items being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    #[inline]
    unsafe fn occupied_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.slices(self.read_end(), self.write_end())
    }

    /// Removes items from the read of ring buffer and drops them.
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
    unsafe fn skip(&self, count_or_all: Option<usize>) -> usize {
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
        self.move_read_end(count);
        count
    }

    /// Sets the new **write** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_write_end` instead.
    unsafe fn set_write_end(&self, value: usize);

    /// Move **write** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in vacant area must be **de-initialized** (dropped) before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of vacant places in the ring buffer.*
    unsafe fn move_write_end(&self, count: usize) {
        debug_assert!(count <= self.vacant_len());
        self.set_write_end((self.write_end() + count) % self.modulus());
    }

    /// Provides a direct access to the ring buffer vacant memory.
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    ///
    /// # Safety
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::move_write_end`] call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    #[inline]
    unsafe fn vacant_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.slices(self.write_end(), self.read_end() + self.capacity().get())
    }
}
