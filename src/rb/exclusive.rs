use super::RbMut;
use crate::traits::{Consumer, Observer, Producer, Raw, RingBuffer};
use core::{marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize};

/// Exclusive access to a standalone buffer through the endpoint traits.
///
/// Holds an owned buffer or a mutable borrow, without changing endpoint-held
/// flags. [`RingBuffer::as_exclusive`] creates the borrowed form.
///
/// Backing-buffer access requires a mutable borrow of the adapter through
/// [`Self::rb_mut`], or consuming it with [`Self::into_inner`]. Shared access does
/// not expose a backing handle, so it cannot create another endpoint while one
/// of the adapter's data views is borrowed.
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
pub struct Exclusive<R: RbMut> {
    inner: R,
    // Shared views expose &Item, so Sync must require Item: Sync even when the buffer's
    // raw storage is Sync for Item: Send.
    _item: PhantomData<<R::Rb as Observer>::Item>,
}

impl<R: RbMut> Exclusive<R> {
    /// Wrap an owned buffer or a mutable borrow with both endpoint roles.
    pub fn new(inner: R) -> Self {
        Self { inner, _item: PhantomData }
    }

    /// Recover the owned buffer or mutable borrow.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Borrow the backing buffer exclusively, for example to use
    /// [`RingBuffer::push_overwrite`].
    ///
    /// This borrow prevents simultaneous access through the adapter and its
    /// data views. The buffer may be replaced through the returned reference.
    pub fn rb_mut(&mut self) -> &mut R::Rb {
        self.inner.rb_mut()
    }
}

impl<R: RbMut> Observer for Exclusive<R> {
    type Item = <R::Rb as Observer>::Item;
    fn capacity(&self) -> NonZeroUsize {
        self.inner.rb().capacity()
    }
    fn read_index(&self) -> usize {
        self.inner.rb().read_index()
    }
    fn write_index(&self) -> usize {
        self.inner.rb().write_index()
    }
    fn read_is_held(&self) -> bool {
        self.inner.rb().read_is_held()
    }
    fn write_is_held(&self) -> bool {
        self.inner.rb().write_is_held()
    }
}

// SAFETY: RbMut grants both roles for the lifetime of the stored handle. All
// operations use the same backing buffer; obtaining a backing handle requires
// a mutable borrow of the adapter or consuming it. No indices or pointers are
// cached across mutable backing-buffer access.
unsafe impl<R: RbMut> Raw for Exclusive<R> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.inner.rb().unsafe_slices(start, end) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.inner.rb().unsafe_slices_mut(start, end) }
    }
    unsafe fn set_read_index(&self, value: usize) {
        unsafe { self.inner.rb().set_read_index(value) };
    }
    unsafe fn set_write_index(&self, value: usize) {
        unsafe { self.inner.rb().set_write_index(value) };
    }
    unsafe fn hold_read(&self, flag: bool) -> bool {
        unsafe { self.inner.rb().hold_read(flag) }
    }
    unsafe fn hold_write(&self, flag: bool) -> bool {
        unsafe { self.inner.rb().hold_write(flag) }
    }
}

impl<R: RbMut> Consumer for Exclusive<R> {}
impl<R: RbMut> Producer for Exclusive<R> {}

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
