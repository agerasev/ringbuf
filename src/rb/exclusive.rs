use crate::traits::{Consumer, Observer, Producer, Raw, RingBuffer};
use core::{marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize};

/// Exclusive access to a standalone buffer through the endpoint traits.
///
/// Created by [`RingBuffer::as_exclusive`]. The mutable borrow owns both roles
/// without changing the endpoint-held flags. This adapter does not expose its
/// backing handle or implement `RingBuffer`, so it cannot be used to create
/// another endpoint while one of its data views is borrowed.
///
/// ```
/// use ringbuf::{StaticRb, traits::*};
/// fn consume<C: Consumer<Item = u8>>(consumer: &mut C) -> Option<u8> {
///     consumer.try_pop()
/// }
/// let mut rb = StaticRb::<u8, 2>::default();
/// rb.try_push(7).unwrap();
/// assert_eq!(consume(&mut rb.as_exclusive()), Some(7));
/// ```
pub struct Exclusive<'a, B: RingBuffer + ?Sized> {
    inner: &'a mut B,
    // Shared views expose &Item, so Sync must require Item: Sync even when B's
    // raw storage is Sync for Item: Send.
    _item: PhantomData<B::Item>,
}

impl<'a, B: RingBuffer + ?Sized> Exclusive<'a, B> {
    pub(crate) fn new(inner: &'a mut B) -> Self {
        Self { inner, _item: PhantomData }
    }
}

impl<B: RingBuffer + ?Sized> Observer for Exclusive<'_, B> {
    type Item = B::Item;
    fn capacity(&self) -> NonZeroUsize {
        self.inner.capacity()
    }
    fn read_index(&self) -> usize {
        self.inner.read_index()
    }
    fn write_index(&self) -> usize {
        self.inner.write_index()
    }
    fn read_is_held(&self) -> bool {
        self.inner.read_is_held()
    }
    fn write_is_held(&self) -> bool {
        self.inner.write_is_held()
    }
}

// SAFETY: the exclusive buffer borrow owns both roles. All operations use the
// same backing buffer, and no safe accessor exposes a competing backing handle.
unsafe impl<B: RingBuffer + ?Sized> Raw for Exclusive<'_, B> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.inner.unsafe_slices(start, end) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.inner.unsafe_slices_mut(start, end) }
    }
    unsafe fn set_read_index(&self, value: usize) {
        unsafe { self.inner.set_read_index(value) };
    }
    unsafe fn set_write_index(&self, value: usize) {
        unsafe { self.inner.set_write_index(value) };
    }
    unsafe fn hold_read(&self, flag: bool) -> bool {
        unsafe { self.inner.hold_read(flag) }
    }
    unsafe fn hold_write(&self, flag: bool) -> bool {
        unsafe { self.inner.hold_write(flag) }
    }
}

impl<B: RingBuffer + ?Sized> Consumer for Exclusive<'_, B> {}
impl<B: RingBuffer + ?Sized> Producer for Exclusive<'_, B> {}

/// Owning iterator for a standalone ring buffer.
pub struct IntoIter<B: RingBuffer> {
    inner: B,
}

impl<B: RingBuffer> IntoIter<B> {
    /// Take ownership of a standalone buffer.
    pub fn new(inner: B) -> Self {
        Self { inner }
    }
    /// Recover the buffer containing the items not yet yielded.
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: RingBuffer> Iterator for IntoIter<B> {
    type Item = B::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.try_pop()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.occupied_len(), None)
    }
}
