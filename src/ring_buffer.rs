use crate::{
    consumer::GlobalConsumer, counter::GlobalCounter, producer::GlobalProducer, utils::uninit_array,
};
use core::{
    cell::UnsafeCell,
    convert::{AsMut, AsRef},
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::Deref,
};

#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

/// Abstract container for the ring buffer.
pub trait Container<T>: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}
impl<T, C> Container<T> for C where C: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}

pub(crate) struct Storage<T, C: Container<T>> {
    len: NonZeroUsize,
    container: UnsafeCell<C>,
    phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for Storage<T, C> where T: Send {}

impl<T, C: Container<T>> Storage<T, C> {
    fn new(mut container: C) -> Self {
        Self {
            len: NonZeroUsize::new(container.as_mut().len()).unwrap(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        self.len
    }
    pub unsafe fn as_slice(&self) -> &[MaybeUninit<T>] {
        (&*self.container.get()).as_ref()
    }
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut()
    }
}

/// Ring buffer itself.
///
/// The structure consists of abstract container (something that could be referenced as contiguous array) and two counters: `head` and `tail`.
/// When an element is extracted from the ring buffer it is taken from the head side. New elements are appended to the tail side.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// This is achieved by using modulus of `2 * Self::capacity()` (instead of `Self::capacity()`) for `head` and `tail` arithmetics.
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head == Self::capacity()` modulo `2 * Self::capacity()`) without using an extra space in container.
pub struct GenericRingBuffer<T, C: Container<T>> {
    pub(crate) storage: Storage<T, C>,
    pub(crate) counter: GlobalCounter,
}

impl<T, C: Container<T>> GenericRingBuffer<T, C> {
    /// The capacity of the ring buffer.
    ///
    /// This value does not change.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.storage.len().get()
    }

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
        GlobalProducer<T, C, Arc<Self>>,
        GlobalConsumer<T, C, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        (GlobalProducer::new(arc.clone()), GlobalConsumer::new(arc))
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you need to store the buffer somewhere.
    pub fn split_static(&mut self) -> (GlobalProducer<T, C, &Self>, GlobalConsumer<T, C, &Self>) {
        (GlobalProducer::new(self), GlobalConsumer::new(self))
    }
}

impl<T, C: Container<T>> Drop for GenericRingBuffer<T, C> {
    fn drop(&mut self) {
        GlobalConsumer::<T, C, &Self>::new(self).acquire().clear();
    }
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T, C: Container<T>>: Deref<Target = GenericRingBuffer<T, C>> {}
#[cfg(feature = "alloc")]
impl<T, C: Container<T>> RingBufferRef<T, C> for Arc<GenericRingBuffer<T, C>> {}
impl<'a, T, C: Container<T>> RingBufferRef<T, C> for &'a GenericRingBuffer<T, C> {}

/// Stack-allocated ring buffer with static capacity.
///
/// Capacity must be greater that zero.
pub type StaticRingBuffer<T, const N: usize> = GenericRingBuffer<T, [MaybeUninit<T>; N]>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type RingBuffer<T> = GenericRingBuffer<T, Vec<MaybeUninit<T>>>;

#[cfg(feature = "alloc")]
impl<T> RingBuffer<T> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

impl<T, const N: usize> Default for StaticRingBuffer<T, N> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}
