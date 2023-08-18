use crate::{producer::AsyncProducer, rb::AsyncRbRef, wrap::AsyncProd};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
#[cfg(feature = "std")]
use futures::io::AsyncWrite;
use futures::{ready, Sink};
use ringbuf::{
    rb::traits::ToRbRef,
    traits::{Observer, Producer, RingBuffer},
};
#[cfg(feature = "std")]
use std::io;

impl<R: AsyncRbRef> Producer for AsyncProd<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base().set_write_index(value)
    }

    #[inline]
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        self.base_mut().try_push(elem)
    }
    #[inline]
    fn push_iter<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> usize {
        self.base_mut().push_iter(iter)
    }
    #[inline]
    fn push_slice(&mut self, elems: &[Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.base_mut().push_slice(elems)
    }
}

impl<R: AsyncRbRef> AsyncProducer for AsyncProd<R> {
    fn register_waker(&self, waker: &core::task::Waker) {
        self.rb().read.register(waker)
    }

    #[inline]
    fn close(&mut self) {
        drop(self.base.take());
    }
}

impl<R: AsyncRbRef> Sink<<R::Target as Observer>::Item> for AsyncProd<R> {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(if ready!(<Self as AsyncProducer>::poll_ready(self, cx)) {
            Ok(())
        } else {
            Err(())
        })
    }
    fn start_send(mut self: Pin<&mut Self>, item: <R::Target as Observer>::Item) -> Result<(), Self::Error> {
        assert!(self.try_push(item).is_ok());
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.close();
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "std")]
impl<R: AsyncRbRef> AsyncWrite for AsyncProd<R>
where
    R::Target: RingBuffer<Item = u8>,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        <Self as AsyncProducer>::poll_write(self, cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.close();
        Poll::Ready(Ok(()))
    }
}
