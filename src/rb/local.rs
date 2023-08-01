use super::{macros::rb_impl_init, traits::RbRef, utils::ranges};
#[cfg(feature = "alloc")]
use crate::traits::Split;
use crate::{
    halves::{Cons, Prod},
    impl_consumer_traits, impl_producer_traits,
    storage::{Shared, Static, Storage},
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
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

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }
}

impl<S: Storage> Producer for LocalRb<S> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<S: Storage> Consumer for LocalRb<S> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}

impl<S: Storage> RingBuffer for LocalRb<S> {}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<S: Storage> AsRef<LocalRb<S>> for LocalRb<S> {
    fn as_ref(&self) -> &LocalRb<S> {
        self
    }
}
unsafe impl<S: Storage> RbRef for LocalRb<S> {
    type Target = LocalRb<S>;
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
impl<S: Storage> SplitRef for LocalRb<S> {
    type RefProd<'a> = Prod<&'a Self> where Self: 'a;
    type RefCons<'a> = Cons<&'a Self> where Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { (Prod::new(self), Cons::new(self)) }
    }
}

rb_impl_init!(LocalRb);

impl_producer_traits!(LocalRb<S: Storage>);
impl_consumer_traits!(LocalRb<S: Storage>);
