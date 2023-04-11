use crate::{
    consumer::{impl_cons_traits, Cons},
    producer::{impl_prod_traits, Prod},
    raw::{RawBase, RawCons, RawProd},
};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{Deref, Range},
    ptr,
};

/// Caching read end of some ring buffer.
///
/// A free space of removed items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// Items inserted by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct CachedCons<R: Deref>
where
    R::Target: RawCons,
{
    base: R,
    read: Cell<usize>,
    write: usize,
}

/// Caching write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct CachedProd<R: Deref>
where
    R::Target: RawProd,
{
    base: R,
    read: usize,
    write: Cell<usize>,
}

impl<R: Deref> RawBase for CachedCons<R>
where
    R::Target: RawCons,
{
    type Item = <R::Target as RawBase>::Item;

    #[inline]
    unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<Self::Item>] {
        self.base.slice(range)
    }
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }
    #[inline]
    fn read_end(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.write
    }
}

impl<R: Deref> RawBase for CachedProd<R>
where
    R::Target: RawProd,
{
    type Item = <R::Target as RawBase>::Item;

    #[inline]
    unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<Self::Item>] {
        self.base.slice(range)
    }
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }
    #[inline]
    fn read_end(&self) -> usize {
        self.read
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.write.get()
    }
}

impl<R: Deref> RawCons for CachedCons<R>
where
    R::Target: RawCons,
{
    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.read.set(value);
    }
}

impl<R: Deref> RawProd for CachedProd<R>
where
    R::Target: RawProd,
{
    #[inline]
    unsafe fn set_write_end(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: Deref> Drop for CachedCons<R>
where
    R::Target: RawCons,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> Drop for CachedProd<R>
where
    R::Target: RawProd,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> CachedCons<R>
where
    R::Target: RawCons,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: Cell::new(base.read_end()),
            write: base.write_end(),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.set_read_end(self.read.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.write = self.base.write_end();
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Commit and destroy `Self` returning underlying consumer.
    pub fn release(self) -> Cons<R> {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { Cons::new(ptr::read(&this.base)) }
    }
}

impl<R: Deref> CachedProd<R>
where
    R::Target: RawProd,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: base.read_end(),
            write: Cell::new(base.write_end()),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.set_write_end(self.write.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.read = self.base.read_end();
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.base.write_end();
        let (first, second) = unsafe { self.base.slices(last_tail, self.write.get()) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.write.set(last_tail);
    }

    /// Commit and destroy `Self` returning underlying producer.
    pub fn release(self) -> Prod<R> {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { Prod::new(ptr::read(&this.base)) }
    }
}

impl_prod_traits!(CachedProd);
impl_cons_traits!(CachedCons);
