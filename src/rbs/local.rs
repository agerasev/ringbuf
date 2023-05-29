use super::{init::rb_impl_init, utils::ranges};
#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    consumer::Consumer,
    halves::direct::{Cons, Prod},
    producer::Producer,
    storage::{Shared, Static, Storage},
    traits::{ring_buffer::Split, Observer, RingBuffer},
};
#[cfg(feature = "alloc")]
use alloc::rc::Rc;
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Ring buffer for single-threaded use only.
pub struct LocalRb<S: Storage> {
    storage: Shared<S>,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<S: Storage> LocalRb<S> {
    /// Constructs ring buffer from storage and indices.
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
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let this = ManuallyDrop::new(self);
        (ptr::read(&this.storage).into_inner(), this.read.get(), this.write.get())
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
    fn vacant_slices(&self) -> (&[MaybeUninit<S::Item>], &[MaybeUninit<S::Item>]) {
        let (first, second) = unsafe { self.unsafe_slices(self.write.get(), self.read.get() + self.capacity().get()) };
        (first as &_, second as &_)
    }
    #[inline]
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        unsafe { self.unsafe_slices(self.write.get(), self.read.get() + self.capacity().get()) }
    }
}

impl<S: Storage> Consumer for LocalRb<S> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }

    #[inline]
    fn occupied_slices(&self) -> (&[MaybeUninit<S::Item>], &[MaybeUninit<S::Item>]) {
        let (first, second) = unsafe { self.unsafe_slices(self.read.get(), self.write.get()) };
        (first as &_, second as &_)
    }
    #[inline]
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        self.unsafe_slices(self.read.get(), self.write.get())
    }
}

impl<S: Storage> RingBuffer for LocalRb<S> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }
}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<'a, S: Storage + 'a> Split for &'a mut LocalRb<S> {
    type Prod = Prod<&'a LocalRb<S>>;
    type Cons = Cons<&'a LocalRb<S>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        unsafe { (Prod::new(self), Cons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage> Split for LocalRb<S> {
    type Prod = Prod<Rc<Self>>;
    type Cons = Cons<Rc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let rc = Rc::new(self);
        unsafe { (Prod::new(rc.clone()), Cons::new(rc)) }
    }
}
impl<S: Storage> LocalRb<S> {
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Prod<Rc<Self>>, Cons<Rc<Self>>) {
        Split::split(self)
    }
    pub fn split_ref(&mut self) -> (Prod<&Self>, Cons<&Self>) {
        Split::split(self)
    }
}

rb_impl_init!(LocalRb);
