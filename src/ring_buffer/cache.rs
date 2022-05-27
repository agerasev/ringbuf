use super::{RingBufferBase, RingBufferRead, RingBufferWrite};
use core::{cell::Cell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

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
        unsafe { self.target.set_head(self.head.get()) }
    }
}

impl<T, R: Deref> Drop for RingBufferWriteCache<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    fn drop(&mut self) {
        unsafe { self.target.set_tail(self.tail.get()) }
    }
}
