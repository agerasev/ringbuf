use crate::{endpoint::AsyncProd, producer::AsyncProducer, rb::AsyncRbHandle};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
#[cfg(feature = "std")]
use futures_util::io::AsyncWrite;
use futures_util::{Sink, ready};
#[cfg(feature = "std")]
use ringbuf::traits::RingBuffer;
use ringbuf::{
    endpoint::Endpoint,
    traits::{
        Observer,
        producer::{DelegateProducer, Producer},
    },
};
#[cfg(feature = "std")]
use std::io;

unsafe impl<R: AsyncRbHandle> DelegateProducer for AsyncProd<R> {}

impl<R: AsyncRbHandle> AsyncProducer for AsyncProd<R> {
    fn is_closed(&self) -> bool {
        !self.base.is_active() || !self.read_is_held()
    }
    fn register_waker(&self, waker: &core::task::Waker) {
        self.rb().markers().read.register(waker)
    }

    #[inline]
    fn close(&mut self) {
        self.base.close();
    }
}

impl<R: AsyncRbHandle> Sink<<R::Rb as Observer>::Item> for AsyncProd<R> {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(if ready!(<Self as AsyncProducer>::poll_ready(self, cx)) {
            Ok(())
        } else {
            Err(())
        })
    }
    fn start_send(mut self: Pin<&mut Self>, item: <R::Rb as Observer>::Item) -> Result<(), Self::Error> {
        self.try_push(item).map_err(|_| ())
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
impl<R: AsyncRbHandle> AsyncWrite for AsyncProd<R>
where
    R::Rb: RingBuffer<Item = u8>,
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

use ringbuf::traits::Presence;
