use super::{
    macros::{rb_impl_consumer_traits, rb_impl_init, rb_impl_producer_traits},
    utils::{assert_capacity, ranges},
};
use crate::traits::Raw;
#[cfg(feature = "alloc")]
use crate::traits::Split;
use crate::{
    storage::Storage,
    traits::{Observer, RingBuffer, SplitRef},
    wrap::{Cons, Prod},
};
#[cfg(feature = "alloc")]
use alloc::{boxed::Box, rc::Rc};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

struct Endpoint {
    index: Cell<usize>,
    held: Cell<bool>,
}

impl Endpoint {
    const fn new(index: usize) -> Self {
        Self {
            index: Cell::new(index),
            held: Cell::new(false),
        }
    }
}

/// Ring buffer for single-threaded use only.
///
/// Slightly faster than multi-threaded version because it doesn't synchronize cache.
pub struct LocalRb<S: Storage + ?Sized> {
    read: Endpoint,
    write: Endpoint,
    storage: S,
}

impl<S: Storage> LocalRb<S> {
    /// Constructs ring buffer from storage and indices.
    ///
    /// Panics if the storage is empty or its length exceeds `usize::MAX / 2`.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see implementation details).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        assert_capacity(storage.len());
        Self {
            storage,
            read: Endpoint::new(read),
            write: Endpoint::new(write),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let this = ManuallyDrop::new(self);
        (unsafe { ptr::read(&this.storage) }, this.read_index(), this.write_index())
    }
}

impl<S: Storage + ?Sized> Observer for LocalRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(self.storage.len()) }
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.index.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.index.get()
    }

    #[inline]
    fn read_is_held(&self) -> bool {
        self.read.held.get()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.write.held.get()
    }
}

impl<S: Storage + ?Sized> RingBuffer for LocalRb<S> {}

// SAFETY: the handle is the buffer itself and always returns that same object.
unsafe impl<S: Storage + ?Sized> super::RbRef for LocalRb<S> {
    type Rb = Self;
    fn rb(&self) -> &Self {
        self
    }
}
// SAFETY: owning this buffer owns both roles; derived borrows prevent moving it
// or borrowing it mutably until those borrows end.
unsafe impl<S: Storage + ?Sized> super::RbMut for LocalRb<S> {
    fn rb_mut(&mut self) -> &mut Self {
        self
    }
}

impl<S: Storage + ?Sized> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(feature = "alloc")]
impl<S: Storage> Split for LocalRb<S> {
    type Prod = Prod<Rc<Self>>;
    type Cons = Cons<Rc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        Rc::new(self).split()
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage + ?Sized> Split for Rc<LocalRb<S>> {
    type Prod = Prod<Self>;
    type Cons = Cons<Self>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        (Prod::new(self.clone()), Cons::new(self))
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage + ?Sized> Split for Box<LocalRb<S>> {
    type Prod = Prod<Rc<LocalRb<S>>>;
    type Cons = Cons<Rc<LocalRb<S>>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        Rc::<LocalRb<S>>::from(self).split()
    }
}
impl<S: Storage + ?Sized> SplitRef for LocalRb<S> {
    type RefProd<'a>
        = Prod<&'a Self>
    where
        Self: 'a;
    type RefCons<'a>
        = Cons<&'a Self>
    where
        Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        (Prod::new(self), Cons::new(self))
    }
}

rb_impl_init!(LocalRb);

rb_impl_producer_traits!(LocalRb<S: Storage>);
rb_impl_consumer_traits!(LocalRb<S: Storage>);

impl<S: Storage + ?Sized> AsRef<Self> for LocalRb<S> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<S: Storage + ?Sized> AsMut<Self> for LocalRb<S> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

// SAFETY: storage and cursor operations preserve the backing buffer
// invariants; callers remain responsible for owning the accessed endpoint.
unsafe impl<S: Storage + ?Sized> Raw for LocalRb<S> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<S::Item>], &[MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        unsafe { (self.storage.slice(first), self.storage.slice(second)) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        unsafe { (self.storage.slice_mut(first), self.storage.slice_mut(second)) }
    }
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.index.set(value);
    }
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.index.set(value);
    }
    #[inline]
    unsafe fn hold_read(&self, flag: bool) -> bool {
        self.read.held.replace(flag)
    }
    #[inline]
    unsafe fn hold_write(&self, flag: bool) -> bool {
        self.write.held.replace(flag)
    }
}

crate::impl_ring_buffer_methods!(LocalRb<S>, [S: Storage + ?Sized]);
