use super::{macros::*, Based};
use crate::traits::{observer::Observe, Consumer, Observer, Producer};
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
pub struct FrozenCons<R: Based>
where
    R::Base: Consumer,
{
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
pub struct FrozenProd<R: Based>
where
    R::Base: Producer,
{
    pub(crate) ref_: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<R: Based> FrozenCons<R>
where
    R::Base: Consumer,
{
    fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}
impl<R: Based> FrozenProd<R>
where
    R::Base: Producer,
{
    fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}

impl<R: Based> Observer for FrozenCons<R>
where
    R::Base: Consumer,
{
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

impl<R: Based> Observer for FrozenProd<R>
where
    R::Base: Producer,
{
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

impl<R: Based> Consumer for FrozenCons<R>
where
    R::Base: Consumer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}

impl<R: Based> Producer for FrozenProd<R>
where
    R::Base: Producer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: Based> Drop for FrozenCons<R>
where
    R::Base: Consumer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Based> Drop for FrozenProd<R>
where
    R::Base: Producer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Based> FrozenCons<R>
where
    R::Base: Consumer,
{
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

impl<R: Based> FrozenProd<R>
where
    R::Base: Producer,
{
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

impl<R: Based> Based for FrozenCons<R>
where
    R::Base: Consumer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: Based> Based for FrozenProd<R>
where
    R::Base: Producer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}

impl<R: Based + Clone> Observe for FrozenProd<R>
where
    R::Base: Producer + Observe,
{
    type Obs = <R::Base as Observe>::Obs;
    fn observe(&self) -> Self::Obs {
        self.base().observe()
    }
}
impl<R: Based + Clone> Observe for FrozenCons<R>
where
    R::Base: Consumer + Observe,
{
    type Obs = <R::Base as Observe>::Obs;
    fn observe(&self) -> Self::Obs {
        self.base().observe()
    }
}
