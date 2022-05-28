use crate::{consumer::Consumer, producer::Producer};
use core::{
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::{Deref, Range},
    ptr, slice,
};

#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

/// Basic ring buffer trait.
///
/// Provides status methods and access to underlying memory.
pub trait RingBufferBase<T> {
    /// Returns underlying raw ring buffer memory as slice.
    ///
    /// # Safety
    ///
    /// All operations on this data must cohere with the counter.
    ///
    /// *Accessing raw data is extremely unsafe.*
    /// It is recommended to use [`Consumer::as_slices()`] and [`Producer::free_space_as_slices()`] instead.
    #[allow(clippy::mut_from_ref)]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>];

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    /// Head position.
    fn head(&self) -> usize;

    /// Tail position.
    fn tail(&self) -> usize;

    /// Modulus for `head` and `tail` values.
    ///
    /// Equals to `2 * len`.
    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.capacity().get()) }
    }

    /// The number of items stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.tail() - self.head()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.head() - self.tail() - self.capacity().get()) % modulus
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
/// Provides reading mechanism and access to occupied memory.
pub trait RingBufferRead<T>: RingBufferBase<T> {
    /// Sets the new **head** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recomended to use `Self::advance_head` instead.
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

    /// Returns a pair of slices which contain, in order, the occupied cells in the ring buffer.
    ///
    /// All items in slices are guaranteed to be **initialized**.
    ///
    /// *The slices may not include items pushed to the buffer by the concurring producer right after this call.*
    fn occupied_ranges(&self) -> (Range<usize>, Range<usize>) {
        let head = self.head();
        let tail = self.tail();
        let len = self.capacity();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        if head_div == tail_div {
            (head_mod..tail_mod, 0..0)
        } else {
            (head_mod..len.get(), 0..tail_mod)
        }
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
    /// *This method must be followed by [`Self::advance`] call with the number of items being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    unsafe fn occupied_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = self.occupied_ranges();
        let ptr = self.data().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
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
        self.advance_head(count);
        count
    }
}

/// Ring buffer write end.
///
/// Provides writing mechanism and access to vacant memory.
pub trait RingBufferWrite<T>: RingBufferBase<T> {
    /// Sets the new **tail** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recomended to use `Self::advance_tail` instead.
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

    /// Returns a pair of slices which contain, in order, the vacant cells in the ring buffer.
    ///
    /// All items in slices are guaranteed to be *un-initialized*.
    ///
    /// *The slices may not include cells freed by the concurring consumer right after this call.*
    fn vacant_ranges(&self) -> (Range<usize>, Range<usize>) {
        let head = self.head();
        let tail = self.tail();
        let len = self.capacity();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        if head_div == tail_div {
            (tail_mod..len.get(), 0..head_mod)
        } else {
            (tail_mod..head_mod, 0..0)
        }
    }

    /// Provides a direct access to the ring buffer vacant memory.
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    ///
    /// # Safety
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by `Self::advance_tail` call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    unsafe fn vacant_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = self.vacant_ranges();
        let ptr = self.data().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }
}

/// The whole ring buffer.
pub trait RingBuffer<T>: RingBufferRead<T> + RingBufferWrite<T> {
    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in [`Arc`]. If you don't want to use heap the see [`Self::split_ref`].
    #[cfg(feature = "alloc")]
    fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>)
    where
        Self: Sized,
    {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
    fn split_ref(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>) {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}
