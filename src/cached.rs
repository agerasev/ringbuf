use crate::{
    consumer::{impl_cons_traits, Cons},
    producer::{impl_prod_traits, Prod, Producer},
    rb::modulus,
    traits::{Consumer, Observer, RingBuffer},
};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::Deref,
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
    R::Target: RingBuffer,
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
    R::Target: RingBuffer,
{
    base: R,
    read: usize,
    write: Cell<usize>,
}

impl<R: Deref> Observer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write - self.read.get()) % modulus
    }
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        (self.capacity().get() + self.read.get() - self.write) % modulus
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.read.get() == self.write
    }
}

impl<R: Deref> Consumer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_read_index(&self, count: usize) {
        self.read.set((self.read.get() + count) % modulus(self));
    }

    #[inline]
    unsafe fn unsafe_occupied_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.unsafe_slices(self.read.get(), self.write)
    }
}

impl<R: Deref> Observer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write.get() - self.read) % modulus
    }
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        (self.capacity().get() + self.read - self.write.get()) % modulus
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.read == self.write.get()
    }
}

impl<R: Deref> Producer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_write_index(&self, count: usize) {
        self.write.set((self.write.get() + count) % modulus(self));
    }

    #[inline]
    unsafe fn unsafe_vacant_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base
            .unsafe_slices(self.write.get(), self.read + self.capacity().get())
    }
}

impl<R: Deref> Drop for CachedCons<R>
where
    R::Target: RingBuffer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> Drop for CachedProd<R>
where
    R::Target: RingBuffer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> CachedCons<R>
where
    R::Target: RingBuffer,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: Cell::new(base.read_index()),
            write: base.write_index(),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.set_read_index(self.read.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.write = self.base.write_index();
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
    R::Target: RingBuffer,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: base.read_index(),
            write: Cell::new(base.write_index()),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe { self.base.set_write_index(self.write.get()) }
    }
    /// Fetch changes to the ring buffer.
    pub fn fetch(&mut self) {
        self.read = self.base.read_index();
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.base.write_index();
        let (first, second) = unsafe { self.base.unsafe_slices(last_tail, self.write.get()) };
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
