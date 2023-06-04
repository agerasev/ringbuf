#![allow(dead_code)]

use super::direct::{Cons, Obs, Prod};
use crate::{
    rb::RbRef,
    traits::{Consumer, Observe, Observer, Producer, RingBuffer},
};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Frozen read end of some ring buffer.
///
/// A free space of removed items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// Items inserted by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
pub struct FrozenCons<R: RbRef> {
    rb: R,
    read: usize,
    write: usize,
}

/// Frozen write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
pub struct FrozenProd<R: RbRef> {
    rb: R,
    read: usize,
    write: usize,
}

impl<R: RbRef> Observer for FrozenCons<R> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.rb().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.rb().unsafe_slices(start, end)
    }
}

impl<R: RbRef> Observer for FrozenProd<R> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.rb().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.rb().unsafe_slices(start, end)
    }
}

impl<R: RbRef> Consumer for FrozenCons<R> {
    #[inline]
    unsafe fn set_read_index(&mut self, value: usize) {
        self.read = value;
    }
}

impl<R: RbRef> Producer for FrozenProd<R> {
    #[inline]
    unsafe fn set_write_index(&mut self, value: usize) {
        self.write = value;
    }
}

impl<R: RbRef> Drop for FrozenCons<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: RbRef> Drop for FrozenProd<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: RbRef> FrozenCons<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb: R) -> Self {
        Self {
            read: rb.deref().read_index(),
            write: rb.deref().write_index(),
            rb,
        }
    }
    pub fn rb(&self) -> &R::Target {
        self.rb.deref()
    }
    pub fn into_rb_ref(self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.rb) }
    }
    /// Commit and destroy `Self` returning underlying consumer.
    pub fn release(self) -> Cons<R> {
        unsafe { Cons::new(self.into_rb_ref()) }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.rb().unsafe_set_read_index(self.read) }
    }
    /// Fetch changes from the ring buffer.
    pub fn fetch(&mut self) {
        self.write = self.rb().write_index();
    }
    /// Commit changes to and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }
}

impl<R: RbRef> FrozenProd<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb: R) -> Self {
        Self {
            read: rb.deref().read_index(),
            write: rb.deref().write_index(),
            rb,
        }
    }
    pub fn rb(&self) -> &R::Target {
        self.rb.deref()
    }
    pub fn into_rb_ref(self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.rb) }
    }
    /// Commit and destroy `Self` returning underlying producer.
    pub fn release(self) -> Prod<R> {
        unsafe { Prod::new(self.into_rb_ref()) }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.rb().unsafe_set_write_index(self.write) }
    }
    /// Fetch changes from the ring buffer.
    pub fn fetch(&mut self) {
        self.read = self.rb().read_index();
    }
    /// Commit changes to and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.rb().write_index();
        let (first, second) = unsafe { self.rb().unsafe_slices(last_tail, self.write) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.write = last_tail;
    }
}

impl<R: RbRef> Observe for FrozenProd<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.rb.clone())
    }
}
impl<R: RbRef> Observe for FrozenCons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.rb.clone())
    }
}
