use crate::{consumer::AsyncConsumer, rb::AsyncRbRef, wrap::AsyncCons};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
#[cfg(feature = "std")]
use futures::io::AsyncRead;
use futures::Stream;
use ringbuf::{
    rb::traits::ToRbRef,
    traits::{Consumer, Observer, RingBuffer},
};
#[cfg(feature = "std")]
use std::io;

impl<R: AsyncRbRef> Consumer for AsyncCons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base().set_read_index(value)
    }
    #[inline]
    fn try_pop(&mut self) -> Option<Self::Item> {
        self.base_mut().try_pop()
    }
    #[inline]
    fn pop_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.base_mut().pop_slice(elems)
    }
}

impl<R: AsyncRbRef> AsyncConsumer for AsyncCons<R> {
    fn register_waker(&self, waker: &core::task::Waker) {
        self.rb().write.register(waker)
    }

    #[inline]
    fn close(&mut self) {
        drop(self.base.take());
    }
}

impl<R: AsyncRbRef> Stream for AsyncCons<R> {
    type Item = <R::Target as Observer>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut waker_registered = false;
        loop {
            let closed = self.is_closed();
            if let Some(item) = self.try_pop() {
                break Poll::Ready(Some(item));
            }
            if closed {
                break Poll::Ready(None);
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

#[cfg(feature = "std")]
impl<R: AsyncRbRef> AsyncRead for AsyncCons<R>
where
    R::Target: RingBuffer<Item = u8>,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let mut waker_registered = false;
        loop {
            let closed = self.is_closed();
            let len = self.pop_slice(buf);
            if len != 0 || closed {
                break Poll::Ready(Ok(len));
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}
