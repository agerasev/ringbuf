use crate::{
    consumer::{Cons, Consumer},
    producer::{Prod, Producer},
    ring_buffer::{ranges, unsafe_occupied_slices, unsafe_vacant_slices, RingBuffer},
    storage::{impl_rb_ctors, Shared, Storage},
    traits::Observer,
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
    /// `read` and `write` positions must be valid (see [`RbBase`](`RbBase`)).
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

impl<S: Storage> Observer for LocalRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }
}

impl<S: Storage> Producer for LocalRb<S> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
    #[inline]
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { unsafe_vacant_slices(self, self) };
        (first as &_, second as &_)
    }
    #[inline]
    fn vacant_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        unsafe { unsafe_vacant_slices(self, self) }
    }
}

impl<S: Storage> Consumer for LocalRb<S> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
    #[inline]
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { unsafe_occupied_slices(self, self) };
        (first as &_, second as &_)
    }
    #[inline]
    unsafe fn occupied_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        unsafe_occupied_slices(self, self)
    }
}

impl<S: Storage> RingBuffer for LocalRb<S> {
    unsafe fn unsafe_slices(
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
}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl_rb_ctors!(LocalRb);
