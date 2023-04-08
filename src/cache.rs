use crate::{raw::RawRb, storage::Shared, stored::StoredRb};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    ptr,
};

/// Ring buffer cache.
///
/// Modifications made by `Self` are not visible for underlying ring buffer until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
///
/// Modifications made by an underlying ring buffer end is not visible for `Self` until [`Self::fetch`]/[`Self::sync`] is called.
pub trait Cache: RawRb {
    type Base: RawRb;

    fn base(&self) -> Self::Base;

    /// Commit changes **into** the ring buffer.
    fn commit(&self);

    /// Fetch updates **from** the ring buffer.
    fn fetch(&mut self);

    /// Commit and fetch at once.
    fn sync(&mut self) {
        self.commit();
        self.fetch();
    }

    /// Slices containing slots that were freed but not yet commited.
    ///
    /// Subset of [`RawRb::vacant_slices`].
    ///
    /// # Safety
    ///
    /// No other access to vacant memory is allowed while holding these slices.
    unsafe fn cached_vacant_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.slices(self.base().read_end(), self.read_end())
    }

    /// Slices containing items that were inserted from but not yet commited.
    ///
    /// Subset of [`RawRb::occupied_slices`].
    ///
    /// # Safety
    ///
    /// No other access to occupied memory is allowed while holding these slices.
    unsafe fn cached_occupied_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.slices(self.base().write_end(), self.write_end())
    }
}

/// Caching write end of some ring buffer.
pub struct ReadCache<R: RawRb + StoredRb> {
    base: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

/// Caching write end of some ring buffer.
///
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct WriteCache<R: RawRb + StoredRb> {
    base: R,
    read: usize,
    write: Cell<usize>,
}

impl<R: RawRb + StoredRb> StoredRb for ReadCache<R> {
    type Storage = R::Storage;

    fn storage(&self) -> &Shared<Self::Storage> {
        self.base.storage()
    }
}

impl<R: RawRb + StoredRb> RawRb for ReadCache<R> {
    #[inline]
    fn read_end(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.write
    }

    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.read.set(value);
    }
    #[inline]
    unsafe fn set_write_end(&self, _value: usize) {
        unimplemented!()
    }
}

impl<R: RawRb + StoredRb> StoredRb for WriteCache<R> {
    type Storage = R::Storage;

    fn storage(&self) -> &Shared<Self::Storage> {
        self.base.storage()
    }
}

impl<R: RawRb + StoredRb> RawRb for WriteCache<R> {
    #[inline]
    fn read_end(&self) -> usize {
        self.read
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.write.get()
    }

    #[inline]
    unsafe fn set_read_end(&self, _value: usize) {
        unimplemented!()
    }
    #[inline]
    unsafe fn set_write_end(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: RawRb + StoredRb> Drop for ReadCache<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: RawRb + StoredRb> Drop for WriteCache<R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<R: RawRb + StoredRb> Cache for ReadCache<R> {
    fn commit(&self) {
        unsafe { self.base.set_read_end(self.read.get()) }
    }
    fn fetch(&mut self) {
        self.write = self.base.write_end();
    }
}

impl<R: RawRb + StoredRb> ReadCache<R> {
    /// Create new ring buffer cache.
    pub fn new(base: R) -> Self {
        Self {
            read: Cell::new(base.read_end()),
            write: base.write_end(),
            base,
        }
    }

    /// Commit and destroy `Self` returning underlying ring buffer.
    pub fn into_inner(mut self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.base) }
    }
}

impl<R: RawRb + StoredRb> Cache for WriteCache<R> {
    fn commit(&self) {
        unsafe { self.base.set_write_end(self.write.get()) }
    }
    fn fetch(&mut self) {
        self.read = self.base.read_end();
    }
}

impl<R: RawRb + StoredRb> WriteCache<R> {
    /// Create new ring buffer cache.
    pub fn new(base: R) -> Self {
        Self {
            read: base.read_end(),
            write: Cell::new(base.write_end()),
            base,
        }
    }

    /// Commit and destroy `Self` returning underlying ring buffer.
    pub fn into_inner(mut self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.base) }
    }

    /// Discard new items pushed since last sync.
    pub fn cached_slices(&mut self) {
        let last_tail = self.base.write_end();
        let (first, second) = unsafe { self.base.slices(last_tail, self.write.get()) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.write.set(last_tail);
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
}
