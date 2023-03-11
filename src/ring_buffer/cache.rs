use super::{RbBase, RbRead, RbRef, RbWrite};
use core::{cell::Cell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ptr};

/// Caching read end of some ring buffer.
///
/// A free space of removed items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// Items inserted by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct RbReadCache<T, R: RbRef>
where
    R::Rb: RbRead<T>,
{
    target: R,
    head: Cell<usize>,
    tail: usize,
    _phantom: PhantomData<T>,
}

/// Caching write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
///
/// Used to implement [`PostponedConsumer`](`crate::consumer::PostponedConsumer`).
pub struct RbWriteCache<T, R: RbRef>
where
    R::Rb: RbWrite<T>,
{
    target: R,
    head: usize,
    tail: Cell<usize>,
    _phantom: PhantomData<T>,
}

impl<T, R: RbRef> RbBase<T> for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    #[inline]
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.target.slices(head, tail)
    }

    #[inline]
    fn capacity_nonzero(&self) -> NonZeroUsize {
        self.target.capacity_nonzero()
    }

    #[inline]
    fn head(&self) -> usize {
        self.head.get()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail
    }
}

impl<T, R: RbRef> RbBase<T> for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    #[inline]
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.target.slices(head, tail)
    }

    #[inline]
    fn capacity_nonzero(&self) -> NonZeroUsize {
        self.target.capacity_nonzero()
    }

    #[inline]
    fn head(&self) -> usize {
        self.head
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.get()
    }
}

impl<T, R: RbRef> RbRead<T> for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, R: RbRef> RbWrite<T> for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, R: RbRef> Drop for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbRef> Drop for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbRef> RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb_ref: R) -> Self {
        Self {
            head: Cell::new(rb_ref.head()),
            tail: rb_ref.tail(),
            target: rb_ref,
            _phantom: PhantomData,
        }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&mut self) {
        unsafe { self.target.set_head(self.head.get()) }
    }

    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.tail = self.target.tail();
    }

    /// Commit and destroy `Self` returning underlying ring buffer.
    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self will not be dropped.
    }
}

impl<T, R: RbRef> RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb_ref: R) -> Self {
        Self {
            head: rb_ref.head(),
            tail: Cell::new(rb_ref.tail()),
            target: rb_ref,
            _phantom: PhantomData,
        }
    }

    /// Commit changes to the ring buffer.
    pub fn commit(&mut self) {
        unsafe { self.target.set_tail(self.tail.get()) }
    }

    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.target.tail();
        let (first, second) = unsafe { self.target.slices(last_tail, self.tail.get()) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.tail.set(last_tail);
    }

    /// Commit changes and fetch updates from the ring buffer.
    pub fn sync(&mut self) {
        self.commit();
        self.head = self.target.head();
    }

    /// Commit and destroy `Self` returning underlying ring buffer.
    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self will not be dropped.
    }
}
