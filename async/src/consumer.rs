use crate::counter::AsyncCounter;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures::stream::Stream;
use ringbuf::{Consumer, RingBuffer, RingBufferRef};

pub struct AsyncConsumer<T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    base: Consumer<T, R>,
}

impl<T, R> AsyncConsumer<T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            base: Consumer::new(ring_buffer),
        }
    }

    pub fn as_sync(&self) -> &Consumer<T, R> {
        &self.base
    }
    pub fn as_mut_sync(&mut self) -> &mut Consumer<T, R> {
        &mut self.base
    }
    pub fn into_sync(self) -> Consumer<T, R> {
        self.base
    }

    pub(crate) fn register_waker(&self, waker: &Waker) {
        self.base
            .ring_buffer()
            .counter()
            .wakers()
            .tail
            .register(waker);
    }

    pub fn pop(&mut self) -> PopFuture<'_, T, R> {
        PopFuture {
            owner: self,
            done: false,
        }
    }

    pub fn stream(&mut self) -> PopStream<'_, T, R> {
        PopStream { owner: self }
    }
}

impl<T, R> AsyncConsumer<T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    pub fn pop_slice<'a: 'b, 'b>(&'a mut self, slice: &'b mut [T]) -> PopSliceFuture<'a, 'b, T, R> {
        PopSliceFuture {
            owner: self,
            slice: Some(slice),
        }
    }
}

pub struct PopFuture<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncConsumer<T, R>,
    done: bool,
}

impl<'a, T, R> Unpin for PopFuture<'a, T, R> where R: RingBufferRef<T, Counter = AsyncCounter> {}

impl<'a, T, R> Future for PopFuture<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(!self.done);
        self.owner.register_waker(cx.waker());
        match self.owner.base.pop() {
            Some(item) => {
                self.done = true;
                Poll::Ready(item)
            }
            None => Poll::Pending,
        }
    }
}

pub struct PopSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncConsumer<T, R>,
    slice: Option<&'b mut [T]>,
}
impl<'a, 'b, T, R> Unpin for PopSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
}
impl<'a, 'b, T, R> Future for PopSliceFuture<'a, 'b, T, R>
where
    T: Copy,
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut slice = self.slice.take().unwrap();
        let len = self.owner.base.pop_slice(slice);
        slice = &mut slice[len..];
        if slice.is_empty() {
            Poll::Ready(())
        } else {
            self.slice.replace(slice);
            Poll::Pending
        }
    }
}

pub struct PopStream<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    owner: &'a mut AsyncConsumer<T, R>,
}
impl<'a, 'b, T, R> Unpin for PopStream<'a, T, R> where R: RingBufferRef<T, Counter = AsyncCounter> {}
impl<'a, 'b, T, R> Stream for PopStream<'a, T, R>
where
    R: RingBufferRef<T, Counter = AsyncCounter>,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.owner.register_waker(cx.waker());
        match self.owner.base.pop() {
            Some(item) => Poll::Ready(Some(item)),
            None => Poll::Pending,
        }
    }
}
