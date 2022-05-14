use super::{Container, RingBuffer, SharedStorage};
use crate::{consumer::Consumer, counter::Counter, producer::Producer};
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Ring buffer itself.
///
/// The structure consists of:
///
/// 1. An abstract container. It is something that could be referenced as contiguous array.
/// 2. A counter. It contains `head` and `tail` positions and implements a logic of moving them.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// For details about how this is achieved see `counter::Counter`.
pub struct OwningRingBuffer<T, C: Container<T>, S: Counter> {
    storage: SharedStorage<T, C>,
    counter: S,
}

impl<T, C: Container<T>, S: Counter> RingBuffer<T> for OwningRingBuffer<T, C, S> {
    type Counter = S;

    #[inline]
    fn capacity(&self) -> usize {
        self.storage.len().get()
    }

    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.storage.as_slice()
    }

    #[inline]
    fn counter(&self) -> &S {
        &self.counter
    }
}

impl<T, C: Container<T>, S: Counter> OwningRingBuffer<T, C, S> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see `counter::Counter`).
    /// Container and counter must have the same `len`.
    pub unsafe fn from_raw_parts(container: C, counter: S) -> Self {
        let storage = SharedStorage::new(container);
        assert_eq!(storage.len(), counter.len());
        Self { counter, storage }
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in `Arc`. If you don't want to use heap the see `split_static`.
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
    pub fn split_static(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>) {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}

impl<T, C: Container<T>, S: Counter> Drop for OwningRingBuffer<T, C, S> {
    fn drop(&mut self) {
        unsafe { Consumer::<T, &Self>::new(self) }.acquire().clear();
    }
}
