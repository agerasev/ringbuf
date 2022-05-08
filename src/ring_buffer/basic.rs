use super::{Container, GlobalCounter, Storage};
use crate::{consumer::GlobalConsumer, producer::GlobalProducer};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

pub trait AbstractRingBuffer<T> {
    /// The capacity of the ring buffer.
    ///
    /// This value does not change.
    fn capacity(&self) -> NonZeroUsize;

    #[allow(clippy::mut_from_ref)]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>];

    fn counter(&self) -> &GlobalCounter;
}

/// Ring buffer itself.
///
/// The structure consists of abstract container (something that could be referenced as contiguous array) and two counters: `head` and `tail`.
/// When an element is extracted from the ring buffer it is taken from the head side. New elements are appended to the tail side.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// This is achieved by using modulus of `2 * Self::capacity()` (instead of `Self::capacity()`) for `head` and `tail` arithmetics.
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head == Self::capacity()` modulo `2 * Self::capacity()`) without using an extra space in container.
pub struct BasicRingBuffer<T, C: Container<T>> {
    storage: Storage<T, C>,
    counter: GlobalCounter,
}

impl<T, C: Container<T>> AbstractRingBuffer<T> for BasicRingBuffer<T, C> {
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.storage.as_slice()
    }

    #[inline]
    fn counter(&self) -> &GlobalCounter {
        &self.counter
    }
}

impl<T, C: Container<T>> BasicRingBuffer<T, C> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see structure documentaton).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        let storage = Storage::new(container);
        Self {
            counter: GlobalCounter::new(storage.len(), head, tail),
            storage,
        }
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in `Arc`. If you don't want to use heap the see `split_static`.
    #[cfg(feature = "alloc")]
    #[allow(clippy::type_complexity)]
    pub fn split(
        self,
    ) -> (
        GlobalProducer<T, Self, Arc<Self>>,
        GlobalConsumer<T, Self, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        unsafe { (GlobalProducer::new(arc.clone()), GlobalConsumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you need to store the buffer somewhere.
    pub fn split_static(
        &mut self,
    ) -> (
        GlobalProducer<T, Self, &Self>,
        GlobalConsumer<T, Self, &Self>,
    ) {
        unsafe { (GlobalProducer::new(self), GlobalConsumer::new(self)) }
    }
}

impl<T, C: Container<T>> Drop for BasicRingBuffer<T, C> {
    fn drop(&mut self) {
        unsafe { GlobalConsumer::<T, Self, &Self>::new(self) }
            .acquire()
            .clear();
    }
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T, B: AbstractRingBuffer<T>>: Deref<Target = B> {}
impl<T, B: AbstractRingBuffer<T>, R> RingBufferRef<T, B> for R where R: Deref<Target = B> {}
