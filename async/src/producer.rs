use crate::ring_buffer::AsyncRbWrite;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures::{future::FusedFuture, never::Never, sink::Sink};
use ringbuf::{ring_buffer::RbRef, Producer};

pub struct AsyncProducer<T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    base: Producer<T, R>,
}

impl<T, R: RbRef> AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    pub fn from_sync(base: Producer<T, R>) -> Self {
        Self { base }
    }
    pub fn into_sync(self) -> Producer<T, R> {
        self.base
    }
    pub fn as_sync(&self) -> &Producer<T, R> {
        &self.base
    }
    pub fn as_mut_sync(&mut self) -> &mut Producer<T, R> {
        &mut self.base
    }

    fn register_waker(&self, waker: &Waker) {
        self.base.rb().register_head_waker(waker);
    }

    pub fn push(&mut self, item: T) -> PushFuture<'_, T, R> {
        PushFuture {
            owner: self,
            item: Some(item),
        }
    }

    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: I) -> PushIterFuture<'_, T, R, I> {
        PushIterFuture {
            owner: self,
            iter: Some(iter),
        }
    }

    pub fn sink(&mut self) -> PushSink<'_, T, R> {
        PushSink {
            owner: self,
            item: None,
        }
    }
}

impl<T: Copy, R: RbRef> AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    pub fn push_slice<'a: 'b, 'b>(&'a mut self, slice: &'b [T]) -> PushSliceFuture<'a, 'b, T, R> {
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
        }
    }
}

pub struct PushFuture<'a, T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    item: Option<T>,
}
impl<'a, T, R: RbRef> Unpin for PushFuture<'a, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, T, R: RbRef> FusedFuture for PushFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.item.is_none()
    }
}
impl<'a, T, R: RbRef> Future for PushFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let item = self.item.take().unwrap();
        self.owner.register_waker(cx.waker());
        match self.owner.base.push(item) {
            Err(item) => {
                self.item.replace(item);
                Poll::Pending
            }
            Ok(()) => Poll::Ready(()),
        }
    }
}

pub struct PushSliceFuture<'a, 'b, T: Copy, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    slice: Option<&'b [T]>,
}
impl<'a, 'b, T: Copy, R: RbRef> Unpin for PushSliceFuture<'a, 'b, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, 'b, T: Copy, R: RbRef> FusedFuture for PushSliceFuture<'a, 'b, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<'a, 'b, T: Copy, R: RbRef> Future for PushSliceFuture<'a, 'b, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut slice = self.slice.take().unwrap();
        let len = self.owner.base.push_slice(slice);
        slice = &slice[len..];
        if slice.is_empty() {
            Poll::Ready(())
        } else {
            self.slice.replace(slice);
            Poll::Pending
        }
    }
}

pub struct PushIterFuture<'a, T, R: RbRef, I: Iterator<Item = T>>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    iter: Option<I>,
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> Unpin for PushIterFuture<'a, T, R, I> where
    R::Rb: AsyncRbWrite<T>
{
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> FusedFuture for PushIterFuture<'a, T, R, I>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.iter.is_none()
    }
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> Future for PushIterFuture<'a, T, R, I>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut iter = self.iter.take().unwrap();
        let iter_ended = {
            let mut local = self.owner.base.postponed();
            local.push_iter(&mut iter);
            !local.is_full()
        };
        if iter_ended {
            Poll::Ready(())
        } else {
            self.iter.replace(iter);
            Poll::Pending
        }
    }
}

pub struct PushSink<'a, T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    item: Option<T>,
}
impl<'a, T, R: RbRef> Unpin for PushSink<'a, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, T, R: RbRef> Sink<T> for PushSink<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Error = Never;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let item = self.item.take().unwrap();
        self.owner.register_waker(cx.waker());
        match self.owner.base.push(item) {
            Err(item) => {
                self.item.replace(item);
                Poll::Pending
            }
            Ok(()) => Poll::Ready(Ok(())),
        }
    }
    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        assert!(self.item.replace(item).is_none());
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
