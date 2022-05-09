use super::Producer;
use crate::ring_buffer::{AbstractAsyncRingBuffer, RingBufferRef};
use core::{
    future::Future,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub struct AsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub(crate) basic: Producer<T, B, R>,
}

impl<T, B, R> AsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            basic: Producer::new(ring_buffer),
        }
    }

    pub fn push(&mut self, item: T) -> PushFuture<'_, T, B, R> {
        PushFuture {
            owner: self,
            item: Some(item),
        }
    }
}

pub struct PushFuture<'a, T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub owner: &'a mut AsyncProducer<T, B, R>,
    pub item: Option<T>,
}

impl<'a, T, B, R> Unpin for PushFuture<'a, T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
}

impl<'a, T, B, R> Future for PushFuture<'a, T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.item.take() {
            None => Poll::Ready(()),
            Some(item) => match self.owner.basic.push(item) {
                Err(item) => {
                    self.item.replace(item);
                    unsafe { self.owner.basic.ring_buffer.wakers() }
                        .pop
                        .register(cx.waker());
                    Poll::Pending
                }
                Ok(()) => {
                    unsafe { self.owner.basic.ring_buffer.wakers() }.push.wake();
                    Poll::Ready(())
                }
            },
        }
    }
}
