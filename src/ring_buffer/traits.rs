use crate::counter::Counter;
use core::{mem::MaybeUninit, ops::Deref};

/// Abstract ring buffer.
pub trait RingBuffer<T> {
    /// Ring buffer counter type.
    type Counter: Counter;

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity must be constant.
    fn capacity(&self) -> usize;

    /// Returns underlying raw ring buffer memory as slice.
    ///
    /// # Safety
    ///
    /// All operations on this data must cohere with the counter.
    ///
    /// *Accessing raw data is extremely unsafe.*
    /// It is recommended to use `Consumer::as_slices()` and `Producer::free_space_as_slices()` instead.
    #[allow(clippy::mut_from_ref)]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>];

    /// Returns the ring buffer counter.
    fn counter(&self) -> &Self::Counter;
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T>: Deref<Target = Self::RingBuffer> {
    /// Underlying ring buffer type.
    type RingBuffer: RingBuffer<T, Counter = Self::Counter>;
    /// Ring buffer counter type. The same as `Self::RingBuffer::Counter`.
    type Counter: Counter;
}

impl<T, B: RingBuffer<T>, R> RingBufferRef<T> for R
where
    R: Deref<Target = B>,
{
    type RingBuffer = B;
    type Counter = B::Counter;
}
