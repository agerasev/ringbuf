use crate::traits::{AsyncConsumer, AsyncObserver, AsyncProducer, AsyncRingBuffer};
use core::{mem::ManuallyDrop, ptr};
use ringbuf::{
    rb::traits::{RbRef, ToRbRef},
    traits::delegate::{self, Delegate, DelegateMut},
    traits::Observe,
    Cons, Obs, Prod,
};

pub struct AsyncObs<R: RbRef>
where
    R::Target: AsyncRingBuffer,
{
    base: Obs<R>,
}
pub struct AsyncProd<R: RbRef>
where
    R::Target: AsyncRingBuffer,
{
    base: Prod<R>,
}
pub struct AsyncCons<R: RbRef>
where
    R::Target: AsyncRingBuffer,
{
    base: Cons<R>,
}

impl<R: RbRef> AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    pub fn new(rb: R) -> Self {
        Self { base: Obs::new(rb) }
    }
}
impl<R: RbRef> AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    pub unsafe fn new(rb: R) -> Self {
        Self { base: Prod::new(rb) }
    }
}
impl<R: RbRef> AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    pub unsafe fn new(rb: R) -> Self {
        Self { base: Cons::new(rb) }
    }
}

impl<R: RbRef> ToRbRef for AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        self.base.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.base.into_rb_ref()
    }
}
impl<R: RbRef> ToRbRef for AsyncProd<R>
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
impl<R: RbRef> ToRbRef for AsyncCons<R>
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

impl<R: RbRef> Unpin for AsyncObs<R> where R::Target: AsyncRingBuffer {}
impl<R: RbRef> Unpin for AsyncProd<R> where R::Target: AsyncRingBuffer {}
impl<R: RbRef> Unpin for AsyncCons<R> where R::Target: AsyncRingBuffer {}

impl<R: RbRef> Delegate for AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    type Base = Obs<R>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
}
impl<R: RbRef> DelegateMut for AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: RbRef> delegate::Observer for AsyncObs<R> where R::Target: AsyncRingBuffer {}

impl<R: RbRef> AsyncObserver for AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    fn is_closed(&self) -> bool {
        self.base.rb().is_closed()
    }
    fn close(&self) {
        self.base.rb().close()
    }
}

impl<R: RbRef> Delegate for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    type Base = Prod<R>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
}
impl<R: RbRef> DelegateMut for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: RbRef> delegate::Observer for AsyncProd<R> where R::Target: AsyncRingBuffer {}
impl<R: RbRef> delegate::Producer for AsyncProd<R> where R::Target: AsyncRingBuffer {}

impl<R: RbRef> AsyncObserver for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    fn is_closed(&self) -> bool {
        self.base.rb().is_closed()
    }
    fn close(&self) {
        self.base.rb().close();
        self.base.rb().wake_consumer();
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

impl<R: RbRef> Delegate for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    type Base = Cons<R>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
}
impl<R: RbRef> DelegateMut for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: RbRef> delegate::Observer for AsyncCons<R> where R::Target: AsyncRingBuffer {}
impl<R: RbRef> delegate::Consumer for AsyncCons<R> where R::Target: AsyncRingBuffer {}

impl<R: RbRef> AsyncObserver for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    fn is_closed(&self) -> bool {
        self.base.rb().is_closed()
    }
    fn close(&self) {
        self.base.rb().close();
        self.base.rb().wake_producer();
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

impl<R: RbRef> Drop for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    fn drop(&mut self) {
        self.close()
    }
}
impl<R: RbRef> Drop for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    fn drop(&mut self) {
        self.close()
    }
}

//impl_producer_traits!(AsyncProd<R: RbRef> where R::Target: AsyncRingBuffer);
//impl_consumer_traits!(AsyncCons<R: RbRef> where R::Target: AsyncRingBuffer);

impl<R: RbRef> Observe for AsyncObs<R>
where
    R::Target: AsyncRingBuffer,
{
    type Obs = AsyncObs<R>;
    fn observe(&self) -> Self::Obs {
        AsyncObs::new(self.base.rb_ref().clone())
    }
}
impl<R: RbRef> Observe for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    type Obs = AsyncObs<R>;
    fn observe(&self) -> Self::Obs {
        AsyncObs::new(self.base.rb_ref().clone())
    }
}
impl<R: RbRef + Observe> Observe for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    type Obs = AsyncObs<R>;
    fn observe(&self) -> Self::Obs {
        AsyncObs::new(self.base.rb_ref().clone())
    }
}
