use crate::{consumer::AsyncConsumer, endpoint::AsyncCons, rb::AsyncRbHandle};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::Stream;
#[cfg(feature = "std")]
use futures_util::io::AsyncRead;
use ringbuf::{
    endpoint::Endpoint,
    traits::{
        Observer,
        consumer::{Consumer, DelegateConsumer},
    },
};
#[cfg(feature = "std")]
use std::io;

unsafe impl<R: AsyncRbHandle> DelegateConsumer for AsyncCons<R> {}

impl<R: AsyncRbHandle> AsyncConsumer for AsyncCons<R> {
    fn is_closed(&self) -> bool {
        !self.base.is_active() || !self.write_is_held()
    }
    fn register_waker(&self, waker: &core::task::Waker) {
        self.rb().markers().write.register(waker)
    }

    #[inline]
    fn close(&mut self) {
        self.base.close();
    }
}

impl<R: AsyncRbHandle> Stream for AsyncCons<R> {
    type Item = <R::Rb as Observer>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.stream_done {
            return Poll::Ready(None);
        }
        let mut waker_registered = false;
        loop {
            let closed = self.is_closed();
            if let Some(item) = self.try_pop() {
                break Poll::Ready(Some(item));
            }
            if closed {
                self.stream_done = true;
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
impl<R: AsyncRbHandle> AsyncRead for AsyncCons<R>
where
    Self: AsyncConsumer<Item = u8>,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        <Self as AsyncConsumer>::poll_read(self, cx, buf)
    }
}

use ringbuf::traits::Presence;

impl<R: AsyncRbHandle> futures_util::stream::FusedStream for AsyncCons<R> {
    fn is_terminated(&self) -> bool {
        self.stream_done
    }
}
