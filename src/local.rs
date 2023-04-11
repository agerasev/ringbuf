use crate::{
    consumer::{Cons, Consumer},
    producer::Prod,
    raw::{ranges, AsRaw, RawRb, RbMarker},
    storage::{impl_rb_ctors, Shared, Storage},
};
#[cfg(feature = "alloc")]
use alloc::rc::Rc;
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Ring buffer for using in single thread.
///
/// Does *not* implement [`Sync`]. And its [`Producer`] and [`Consumer`] do *not* implement [`Send`].
pub struct LocalRb<S: Storage> {
    storage: Shared<S>,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<S: Storage> LocalRb<S> {
    /// Constructs ring buffer from storage and counters.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read: Cell::new(read),
            write: Cell::new(write),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` counters.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let (read, write) = (self.read_index(), self.write_index());
        let self_ = ManuallyDrop::new(self);
        (ptr::read(&self_.storage).into_inner(), read, write)
    }

    pub fn split_ref(&mut self) -> (Prod<&Self>, Cons<&Self>) {
        unsafe { (Prod::new(self), Cons::new(self)) }
    }
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Prod<Rc<Self>>, Cons<Rc<Self>>) {
        let rc = Rc::new(self);
        unsafe { (Prod::new(rc.clone()), Cons::new(rc)) }
    }
}

impl<S: Storage> RawRb for LocalRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }

    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<S: Storage> AsRaw for LocalRb<S> {
    type Raw = Self;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}
unsafe impl<S: Storage> RbMarker for LocalRb<S> {}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl_rb_ctors!(LocalRb);
