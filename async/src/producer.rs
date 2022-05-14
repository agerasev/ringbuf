use crate::counter::AsyncCounter;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures::{future::FusedFuture, never::Never, sink::Sink};
use ringbuf::{Producer, RingBuffer, RingBufferRef};

pub struct AsyncProducer<T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    base: Producer<T, R>,
}

impl<T, R> AsyncProducer<T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            base: Producer::new(ring_buffer),
        }
    }

    pub fn as_sync(&self) -> &Producer<T, R> {
        &self.base
    }
    pub fn as_mut_sync(&mut self) -> &mut Producer<T, R> {
        &mut self.base
    }
    pub fn into_sync(self) -> Producer<T, R> {
        self.base
    }

    pub(crate) fn register_waker(&self, waker: &Waker) {
        self.base
            .ring_buffer()
            .counter()
            .wakers()
            .head
            .register(waker);
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
        PushSink { owner: self }
    }
}

impl<T, R> AsyncProducer<T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    pub fn push_slice<'a: 'b, 'b>(&'a mut self, slice: &'b [T]) -> PushSliceFuture<'a, 'b, T, R> {
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
        }
    }
}

pub struct PushFuture<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncProducer<T, R>,
    item: Option<T>,
}
impl<'a, T, R> Unpin for PushFuture<'a, T, R> where R: RingBufferRef<T, Counter = AsyncCounter> {}
impl<'a, T, R> FusedFuture for PushFuture<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    fn is_terminated(&self) -> bool {
        self.item.is_none()
    }
}
impl<'a, T, R> Future for PushFuture<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
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

pub struct PushSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncProducer<T, R>,
    slice: Option<&'b [T]>,
}
impl<'a, 'b, T, R> Unpin for PushSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
}
impl<'a, 'b, T, R> FusedFuture for PushSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<'a, 'b, T, R> Future for PushSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
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

pub struct PushIterFuture<'a, T, R, I>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
    I: Iterator<Item = T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    iter: Option<I>,
}
impl<'a, T, R, I> Unpin for PushIterFuture<'a, T, R, I>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
    I: Iterator<Item = T>,
{
}
impl<'a, T, R, I> FusedFuture for PushIterFuture<'a, T, R, I>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
    I: Iterator<Item = T>,
{
    fn is_terminated(&self) -> bool {
        self.iter.is_none()
    }
}
impl<'a, T, R, I> Future for PushIterFuture<'a, T, R, I>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
    I: Iterator<Item = T>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut iter = self.iter.take().unwrap();
        let iter_ended = {
            let mut local = self.owner.base.acquire();
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

pub struct PushSink<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncProducer<T, R>,
}
impl<'a, T, R> Unpin for PushSink<'a, T, R> where R: RingBufferRef<T, Counter = AsyncCounter> {}
impl<'a, T, R> Sink<T> for PushSink<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    type Error = Never;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.owner.register_waker(cx.waker());
        if self.owner.base.is_full() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        assert!(self.owner.base.push(item).is_ok());
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
