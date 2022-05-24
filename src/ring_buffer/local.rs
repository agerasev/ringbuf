use super::{Container, RingBuffer, RingBufferBase, RingBufferHead, RingBufferTail, SharedStorage};
//use crate::{consumer::Consumer, counter::Counter, producer::Producer};
use core::{cell::Cell, mem::MaybeUninit, num::NonZeroUsize};

//#[cfg(feature = "alloc")]
//use alloc::sync::Arc;

/// Ring buffer itself.
///
/// The structure consists of:
///
/// 1. An abstract container. It is something that could be referenced as contiguous array.
/// 2. A counter. It contains `head` and `tail` positions and implements a logic of moving them.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// For details about how this is achieved see `counter::Counter`.
pub struct LocalRingBuffer<T, C: Container<T>> {
    storage: SharedStorage<T, C>,
    len: NonZeroUsize,
    head: Cell<usize>,
    tail: Cell<usize>,
}

impl<T, C: Container<T>> RingBufferBase<T> for LocalRingBuffer<T, C> {
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.storage.as_slice()
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.len
    }

    #[inline]
    fn head(&self) -> usize {
        self.head.get()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.get()
    }
}

impl<T, C: Container<T>> RingBufferHead<T> for LocalRingBuffer<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, C: Container<T>> RingBufferTail<T> for LocalRingBuffer<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, C: Container<T>> RingBuffer<T> for LocalRingBuffer<T, C> {}

impl<T, C: Container<T>> LocalRingBuffer<T, C> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see [`Counter`](`crate::counter::Counter`)).
    ///
    /// Container and counter must have the same `len`.
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        let storage = SharedStorage::new(container);
        Self {
            len: storage.len(),
            storage,
            head: Cell::new(head),
            tail: Cell::new(tail),
        }
    }
    /*
    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in [`Arc`]. If you don't want to use heap the see [`Self::split_static`].
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
    */
}

/*
impl<T, C: Container<T>> Drop for LocalRingBuffer<T, C> {
    fn drop(&mut self) {
        unsafe { Consumer::<T, &Self>::new(self) }.acquire().clear();
    }
}
*/
