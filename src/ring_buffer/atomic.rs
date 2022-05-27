use super::{
    Container, RingBuffer, RingBufferBase, RingBufferRead, RingBufferWrite, SharedStorage,
};
use cache_padded::CachePadded;
use core::{
    mem::MaybeUninit,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Ring buffer that could be shared between threads.
pub struct AtomicRingBuffer<T, C: Container<T>> {
    storage: SharedStorage<T, C>,
    len: NonZeroUsize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl<T, C: Container<T>> RingBufferBase<T> for AtomicRingBuffer<T, C> {
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

impl<T, C: Container<T>> RingBufferRead<T> for AtomicRingBuffer<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> RingBufferWrite<T> for AtomicRingBuffer<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> RingBuffer<T> for AtomicRingBuffer<T, C> {}

impl<T, C: Container<T>> Drop for AtomicRingBuffer<T, C> {
    fn drop(&mut self) {
        unsafe { self.clear() };
    }
}

impl<T, C: Container<T>> AtomicRingBuffer<T, C> {
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
}
