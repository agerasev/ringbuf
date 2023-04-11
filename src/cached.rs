use crate::{
    consumer::{impl_cons_traits, Cons},
    producer::{impl_prod_traits, Prod},
    raw::{AsRaw, ConsMarker, ProdMarker, RawRb},
};
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
pub struct CachedCons<R: AsRaw> {
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
pub struct CachedProd<R: AsRaw> {
    base: R,
    read: usize,
    write: Cell<usize>,
}

impl<R: AsRaw> RawRb for CachedCons<R> {
    type Item = <R::Raw as RawRb>::Item;

    #[inline]
    unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.as_raw().slices(start, end)
    }
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.as_raw().capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write
    }
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
    #[inline]
    unsafe fn set_write_index(&self, _: usize) {
        unimplemented!();
    }
}

impl<R: AsRaw> RawRb for CachedProd<R> {
    type Item = <R::Raw as RawRb>::Item;

    #[inline]
    unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.as_raw().slices(start, end)
    }
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.as_raw().capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.read
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }
    #[inline]
    unsafe fn set_read_index(&self, _: usize) {
        unimplemented!();
    }
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: AsRaw> Drop for CachedCons<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: AsRaw> Drop for CachedProd<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: AsRaw> AsRaw for CachedCons<R> {
    type Raw = Self;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}

impl<R: AsRaw> AsRaw for CachedProd<R> {
    type Raw = Self;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}

unsafe impl<R: AsRaw> ConsMarker for CachedCons<R> {}
unsafe impl<R: AsRaw> ProdMarker for CachedProd<R> {}

impl<R: AsRaw> CachedCons<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: Cell::new(base.as_raw().read_index()),
            write: base.as_raw().write_index(),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.as_raw().set_read_index(self.read.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.write = self.base.as_raw().write_index();
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

impl<R: AsRaw> CachedProd<R> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: base.as_raw().read_index(),
            write: Cell::new(base.as_raw().write_index()),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.as_raw().set_write_index(self.write.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.read = self.base.as_raw().read_index();
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.base.as_raw().write_index();
        let (first, second) = unsafe { self.base.as_raw().slices(last_tail, self.write.get()) };
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
