use super::{Container, Rb, RbBase, RbRead, RbWrite, SharedStorage};
use crate::{consumer::Consumer, producer::Producer};
use cache_padded::CachePadded;
use core::{
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Ring buffer that could be shared between threads.
pub struct SharedRb<T, C: Container<T>> {
    storage: SharedStorage<T, C>,
    len: NonZeroUsize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl<T, C: Container<T>> RbBase<T> for SharedRb<T, C> {
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
        self.head.load(Ordering::Acquire)
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }
}

impl<T, C: Container<T>> RbRead<T> for SharedRb<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> RbWrite<T> for SharedRb<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> Rb<T> for SharedRb<T, C> {}

impl<T, C: Container<T>> Drop for SharedRb<T, C> {
    fn drop(&mut self) {
        unsafe { self.skip(None) };
    }
}

impl<T, C: Container<T>> SharedRb<T, C> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see [`Counter`](`crate::counter::Counter`)).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        let storage = SharedStorage::new(container);
        Self {
            len: storage.len(),
            storage,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    /// Destructures ring buffer into underlying container and `head` and `tail` counters.
    ///
    /// # Safety
    ///
    /// Initialized contents of the container must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (C, usize, usize) {
        let (head, tail) = (self.head(), self.tail());
        let self_uninit = MaybeUninit::new(self);
        let self_ref = self_uninit.assume_init_ref();

        // In case if `AtomicUsize` implements `Drop`.
        mem::drop(ptr::read(&self_ref.head));
        mem::drop(ptr::read(&self_ref.tail));

        (ptr::read(&self_ref.storage).into_inner(), head, tail)
        // `Self::drop` is not called.
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in [`Arc`]. If you don't want to use heap the see [`Self::split_ref`].
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>)
    where
        Self: Sized,
    {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
    pub fn split_ref(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>)
    where
        Self: Sized,
    {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}
