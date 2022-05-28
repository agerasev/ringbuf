use super::{RingBufferBase, RingBufferRead, RingBufferWrite};
use core::{cell::Cell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ops::Deref, ptr};

pub struct RingBufferReadCache<T, R: Deref>
where
    R::Target: RingBufferRead<T>,
{
    target: R,
    head: Cell<usize>,
    tail: usize,
    _phantom: PhantomData<T>,
}

pub struct RingBufferWriteCache<T, R: Deref>
where
    R::Target: RingBufferWrite<T>,
{
    target: R,
    head: usize,
    tail: Cell<usize>,
    _phantom: PhantomData<T>,
}

impl<T, R: Deref> RingBufferBase<T> for RingBufferReadCache<T, R>
where
    R::Target: RingBufferRead<T>,
{
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.target.data()
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.target.capacity()
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

impl<T, R: Deref> RingBufferBase<T> for RingBufferWriteCache<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.target.data()
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.target.capacity()
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

impl<T, R: Deref> RingBufferRead<T> for RingBufferReadCache<T, R>
where
    R::Target: RingBufferRead<T>,
{
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, R: Deref> RingBufferWrite<T> for RingBufferWriteCache<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, R: Deref> Drop for RingBufferReadCache<T, R>
where
    R::Target: RingBufferRead<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: Deref> Drop for RingBufferWriteCache<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: Deref> RingBufferReadCache<T, R>
where
    R::Target: RingBufferRead<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            head: Cell::new(ring_buffer.head()),
            tail: ring_buffer.tail(),
            target: ring_buffer,
            _phantom: PhantomData,
        }
    }

    pub fn commit(&mut self) {
        unsafe { self.target.set_head(self.head.get()) }
    }

    pub fn sync(&mut self) {
        self.commit();
        self.tail = self.target.tail();
    }

    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self::drop is not called.
    }
}

impl<T, R: Deref> RingBufferWriteCache<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            head: ring_buffer.head(),
            tail: Cell::new(ring_buffer.tail()),
            target: ring_buffer,
            _phantom: PhantomData,
        }
    }

    pub fn commit(&mut self) {
        unsafe { self.target.set_tail(self.tail.get()) }
    }

    pub fn sync(&mut self) {
        self.commit();
        self.head = self.target.head();
    }

    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self::drop is not called.
    }
}
