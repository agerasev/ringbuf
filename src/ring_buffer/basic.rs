use super::{Container, SharedStorage};
use crate::{consumer::Consumer, counter::Counter, producer::Producer};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Ring buffer itself.
///
/// The structure consists of abstract container (something that could be referenced as contiguous array) and two counters: `head` and `tail`.
/// When an element is extracted from the ring buffer it is taken from the head side. New elements are appended to the tail side.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// This is achieved by using modulus of `2 * Self::capacity()` (instead of `Self::capacity()`) for `head` and `tail` arithmetics.
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head == Self::capacity()` modulo `2 * Self::capacity()`) without using an extra space in container.
pub struct RingBuffer<T, C: Container<T>, S: Counter> {
    storage: SharedStorage<T, C>,
    counter: S,
}

impl<T, C: Container<T>, S: Counter> RingBuffer<T, C, S> {
    #[inline]
    pub fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.storage.as_slice()
    }

    #[inline]
    pub fn counter(&self) -> &S {
        &self.counter
    }

    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see structure documentaton).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        let storage = SharedStorage::new(container);
        Self {
            counter: S::new(storage.len(), head, tail),
            storage,
        }
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in `Arc`. If you don't want to use heap the see `split_static`.
    #[cfg(feature = "alloc")]
    #[allow(clippy::type_complexity)]
    pub fn split(self) -> (Producer<T, C, S, Arc<Self>>, Consumer<T, C, S, Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you need to store the buffer somewhere.
    pub fn split_static(&mut self) -> (Producer<T, C, S, &Self>, Consumer<T, C, S, &Self>) {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}

impl<T, C: Container<T>, S: Counter> Drop for RingBuffer<T, C, S> {
    fn drop(&mut self) {
        unsafe { Consumer::<T, C, S, &Self>::new(self) }
            .acquire()
            .clear();
    }
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T, C: Container<T>, S: Counter>:
    Deref<Target = RingBuffer<T, C, S>>
{
}
impl<T, C: Container<T>, S: Counter, R> RingBufferRef<T, C, S> for R where
    R: Deref<Target = RingBuffer<T, C, S>>
{
}
