use crate::{consumer::AsyncConsumer, rb::AsyncRbRef, wrap::AsyncCons};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::Stream;
#[cfg(feature = "std")]
use futures_util::io::AsyncRead;
use ringbuf::{
    traits::{
        Observer,
        consumer::{Consumer, DelegateConsumer},
    },
    wrap::Wrap,
};
#[cfg(feature = "std")]
use std::io;

impl<R: AsyncRbRef> DelegateConsumer for AsyncCons<R> {}

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
    type Item = <R::Rb as Observer>::Item;

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
    Self: AsyncConsumer<Item = u8>,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        <Self as AsyncConsumer>::poll_read(self, cx, buf)
    }
}
