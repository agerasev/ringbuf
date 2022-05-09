use super::Consumer;
use crate::{
    counter::AsyncCounter,
    ring_buffer::{Container, RingBufferRef},
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub struct AsyncConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub(crate) base: Consumer<T, C, AsyncCounter, R>,
}

impl<T, C, R> AsyncConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            base: Consumer::new(ring_buffer),
        }
    }

    fn register_waker(&self, waker: &Waker) {
        self.base
            .ring_buffer
            .counter()
            .wakers()
            .tail
            .register(waker);
    }

    pub fn pop(&mut self) -> PopFuture<'_, T, C, R> {
        PopFuture {
            owner: self,
            done: false,
        }
    }
}

impl<T, C, R> AsyncConsumer<T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    pub fn pop_slice<'a: 'b, 'b>(
        &'a mut self,
        slice: &'b mut [T],
    ) -> PopSliceFuture<'a, 'b, T, C, R> {
        PopSliceFuture {
            owner: self,
            slice: Some(slice),
        }
    }
}

pub struct PopFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    owner: &'a mut AsyncConsumer<T, C, R>,
    done: bool,
}

impl<'a, T, C, R> Unpin for PopFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
}

impl<'a, T, C, R> Future for PopFuture<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
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

pub struct PopSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
    owner: &'a mut AsyncConsumer<T, C, R>,
    slice: Option<&'b mut [T]>,
}
impl<'a, 'b, T, C, R> Unpin for PopSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
{
}
impl<'a, 'b, T, C, R> Future for PopSliceFuture<'a, 'b, T, C, R>
where
    T: Copy,
    C: Container<T>,
    R: RingBufferRef<T, C, AsyncCounter>,
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
