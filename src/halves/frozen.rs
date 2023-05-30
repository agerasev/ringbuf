use super::{
    based::{ConsRef, ProdRef},
    macros::*,
};
use crate::traits::{Consumer, Observer, Producer};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Caching read end of some ring buffer.
///
/// A free space of removed items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// Items inserted by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct FrozenCons<R: ConsRef> {
    pub(crate) ref_: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

/// Caching write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct FrozenProd<R: ProdRef> {
    pub(crate) ref_: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<R: ConsRef> FrozenCons<R> {
    fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}
impl<R: ProdRef> FrozenProd<R> {
    fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}

impl<R: ConsRef> Observer for FrozenCons<R> {
    type Item = <R::Base as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.base().unsafe_slices(start, end)
    }
}

impl<R: ProdRef> Observer for FrozenProd<R> {
    type Item = <R::Base as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.base().unsafe_slices(start, end)
    }
}

impl<R: ConsRef> Consumer for FrozenCons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}

impl<R: ProdRef> Producer for FrozenProd<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: ConsRef> Drop for FrozenCons<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: ProdRef> Drop for FrozenProd<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: ConsRef> FrozenCons<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            read: Cell::new(ref_.base_deref().read_index()),
            write: Cell::new(ref_.base_deref().write_index()),
            ref_,
        }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base().set_read_index(self.read.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&self) {
        self.write.set(self.base().write_index());
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&self) {
        self.commit();
        self.fetch();
    }
    /// Commit and destroy `Self` returning underlying consumer.
    pub fn release(self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.ref_) }
    }
}

impl<R: ProdRef> FrozenProd<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            read: Cell::new(ref_.base_deref().read_index()),
            write: Cell::new(ref_.base_deref().write_index()),
            ref_,
        }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base().set_write_index(self.write.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&self) {
        self.read.set(self.base().read_index());
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&self) {
        self.commit();
        self.fetch();
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.base().write_index();
        let (first, second) = unsafe { self.base().unsafe_slices(last_tail, self.write.get()) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.write.set(last_tail);
    }
    /// Commit and destroy `Self` returning underlying producer.
    pub fn release(self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.ref_) }
    }
}

impl_prod_traits!(FrozenProd);
impl_cons_traits!(FrozenCons);

impl<R: ProdRef> ProdRef for FrozenProd<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: ConsRef> ConsRef for FrozenCons<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
