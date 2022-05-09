use super::Producer;
use crate::{
    counter::AsyncCounter,
    ring_buffer::{Container, RingBufferRef},
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub struct AsyncProducer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub(crate) base: Producer<T, C, AsyncCounter, R>,
}

impl<T, C, R> AsyncProducer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            base: Producer::new(ring_buffer),
        }
    }

    fn register_waker(&self, waker: &Waker) {
        self.base
            .ring_buffer
            .counter()
            .wakers()
            .head
            .register(waker);
    }

    pub fn push(&mut self, item: T) -> PushFuture<'_, T, C, R> {
        PushFuture {
            owner: self,
            item: Some(item),
        }
    }
}

impl<T, C, R> AsyncProducer<T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub fn push_slice<'a: 'b, 'b>(
        &'a mut self,
        slice: &'b [T],
    ) -> PushSliceFuture<'a, 'b, T, C, R> {
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
        }
    }
}

pub struct PushFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    owner: &'a mut AsyncProducer<T, C, R>,
    item: Option<T>,
}
impl<'a, T, C, R> Unpin for PushFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
}
impl<'a, T, C, R> Future for PushFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
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

pub struct PushSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    owner: &'a mut AsyncProducer<T, C, R>,
    slice: Option<&'b [T]>,
}
impl<'a, 'b, T, C, R> Unpin for PushSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
}
impl<'a, 'b, T, C, R> Future for PushSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
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
