use crate::traits::{AsyncConsumer, AsyncObserver, AsyncProducer, AsyncRingBuffer};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};
use ringbuf::{
    rb::traits::{RbRef, ToRbRef},
    traits::{Consumer, Observer, Producer},
    wrap::caching::Caching,
    Obs,
};

pub struct AsyncWrap<R: RbRef, const P: bool, const C: bool>
where
    R::Target: AsyncRingBuffer,
{
    base: Caching<R, P, C>,
}

pub type AsyncProd<R> = AsyncWrap<R, true, false>;
pub type AsyncCons<R> = AsyncWrap<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> AsyncWrap<R, P, C>
where
    R::Target: AsyncRingBuffer,
{
    pub unsafe fn new(rb: R) -> Self {
        Self { base: Caching::new(rb) }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base.observe()
    }
}

impl<R: RbRef, const P: bool, const C: bool> ToRbRef for AsyncWrap<R, P, C>
where
    R::Target: AsyncRingBuffer,
{
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        self.base.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        let this = ManuallyDrop::new(self);
        this.close();
        unsafe { ptr::read(&this.base) }.into_rb_ref()
    }
}

impl<R: RbRef, const P: bool, const C: bool> Unpin for AsyncWrap<R, P, C> where R::Target: AsyncRingBuffer {}

impl<R: RbRef, const P: bool, const C: bool> Observer for AsyncWrap<R, P, C>
where
    R::Target: AsyncRingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.base.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.base.write_index()
    }
    #[inline]
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.base.unsafe_slices(start, end)
    }
}

impl<R: RbRef, const P: bool, const C: bool> AsyncObserver for AsyncWrap<R, P, C>
where
    R::Target: AsyncRingBuffer,
{
    fn is_closed(&self) -> bool {
        self.base.rb().is_closed()
    }
    fn close(&self) {
        self.base.rb().close();
        if P {
            self.base.rb().wake_consumer();
        }
        if C {
            self.base.rb().wake_producer();
        }
    }
}

impl<R: RbRef> Producer for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value)
    }
    #[inline]
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        self.base.try_push(elem)
    }
    #[inline]
    fn push_iter<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> usize {
        self.base.push_iter(iter)
    }
    #[inline]
    fn push_slice(&mut self, elems: &[Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.base.push_slice(elems)
    }
}

impl<R: RbRef> AsyncProducer for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    fn register_read_waker(&self, waker: &core::task::Waker) {
        self.base.rb().register_read_waker(waker)
    }
}
impl<R: RbRef> Consumer for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value)
    }
    #[inline]
    fn try_pop(&mut self) -> Option<Self::Item> {
        self.base.try_pop()
    }
    #[inline]
    fn pop_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.base.pop_slice(elems)
    }
}

impl<R: RbRef> AsyncConsumer for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    fn register_write_waker(&self, waker: &core::task::Waker) {
        self.base.rb().register_write_waker(waker)
    }
}

impl<R: RbRef, const P: bool, const C: bool> Drop for AsyncWrap<R, P, C>
where
    R::Target: AsyncRingBuffer,
{
    fn drop(&mut self) {
        if P || C {
            self.close()
        }
    }
}
