use super::{
    Container, RingBuffer, RingBufferBase, RingBufferRead, RingBufferWrite, SharedStorage,
};
use core::{cell::Cell, mem::MaybeUninit, num::NonZeroUsize};

/// Ring buffer for using in single thread.
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

impl<T, C: Container<T>> RingBufferRead<T> for LocalRingBuffer<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, C: Container<T>> RingBufferWrite<T> for LocalRingBuffer<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, C: Container<T>> RingBuffer<T> for LocalRingBuffer<T, C> {}

impl<T, C: Container<T>> Drop for LocalRingBuffer<T, C> {
    fn drop(&mut self) {
        unsafe { self.skip(None) };
    }
}

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
}
