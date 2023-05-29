use crate::traits::{Consumer, Observer, Producer, RingBuffer};
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
pub struct FrozenCons<R: Deref>
where
    R::Target: RingBuffer,
{
    pub(crate) base: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

/// Caching write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct FrozenProd<R: Deref>
where
    R::Target: RingBuffer,
{
    pub(crate) base: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<R: Deref> Observer for FrozenCons<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
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

impl<R: Deref> Observer for FrozenProd<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
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

impl<R: Deref> Consumer for FrozenCons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }

    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        let (first, second) = unsafe { rb.unsafe_slices(rb.read_index(), rb.write_index()) };
        (first as &_, second as &_)
    }
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        rb.unsafe_slices(rb.read_index(), rb.write_index())
    }
}

impl<R: Deref> Producer for FrozenProd<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }

    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        let (first, second) = unsafe { rb.unsafe_slices(rb.write_index(), rb.read_index() + rb.capacity().get()) };
        (first as &_, second as &_)
    }
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        unsafe { rb.unsafe_slices(rb.write_index(), rb.read_index() + rb.capacity().get()) }
    }
}

impl<R: Deref> Drop for FrozenCons<R>
where
    R::Target: RingBuffer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> Drop for FrozenProd<R>
where
    R::Target: RingBuffer,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: Deref> FrozenCons<R>
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
            write: Cell::new(base.write_index()),
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
    pub fn fetch(&self) {
        self.write.set(self.base.write_index());
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&self) {
        self.commit();
        self.fetch();
    }
    /*
    /// Commit and destroy `Self` returning underlying consumer.
    pub fn release(self) -> Cons<R> {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { Cons::new(ptr::read(&this.base)) }
    }
    */
}

impl<R: Deref> FrozenProd<R>
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
    pub fn fetch(&self) {
        self.read.set(self.base.read_index());
    }
    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&self) {
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
    /*
    /// Commit and destroy `Self` returning underlying producer.
    pub fn release(self) -> Prod<R> {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { Prod::new(ptr::read(&this.base)) }
    }
    */
}

//impl_prod_traits!(FrozenProd);
//impl_cons_traits!(FrozenCons);
