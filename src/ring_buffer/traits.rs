use crate::counter::Counter;
use core::{mem::MaybeUninit, ops::Deref};

pub trait RingBuffer<T> {
    type Counter: Counter;

    fn capacity(&self) -> usize;
    #[allow(clippy::mut_from_ref)]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>];
    fn counter(&self) -> &Self::Counter;
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T>: Deref<Target = Self::RingBuffer> {
    type RingBuffer: RingBuffer<T, Counter = Self::Counter>;
    type Counter: Counter;
}

impl<T, B: RingBuffer<T>, R> RingBufferRef<T> for R
where
    R: Deref<Target = B>,
{
    type RingBuffer = B;
    type Counter = B::Counter;
}
