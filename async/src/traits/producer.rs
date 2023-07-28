use crate::halves::AsyncProd;

use super::{AsyncObserver, AsyncRingBuffer};
use core::{
    future::Future,
    iter::Peekable,
    pin::Pin,
    task::{Context, Poll, Waker},
};
#[cfg(feature = "std")]
use futures::io::AsyncWrite;
use futures::{future::FusedFuture, Sink};
use ringbuf::{
    rb::traits::RbRef,
    traits::{Observer, Producer},
};
#[cfg(feature = "std")]
use std::io;

pub trait AsyncProducer: AsyncObserver + Producer {
    fn register_read_waker(&self, waker: &Waker);

    /// Push item to the ring buffer waiting asynchronously if the buffer is full.
    ///
    /// Future returns:
    /// + `Ok` - item successfully pushed.
    /// + `Err(item)` - the corresponding consumer was dropped, item is returned back.
    fn push(&mut self, item: Self::Item) -> PushFuture<'_, Self> {
        PushFuture {
            owner: self,
            item: Some(item),
        }
    }

    /// Push items from iterator waiting asynchronously if the buffer is full.
    ///
    /// Future returns:
    /// + `Ok` - iterator ended.
    /// + `Err(iter)` - the corresponding consumer was dropped, remaining iterator is returned back.
    fn push_iter_all<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> PushIterFuture<'_, Self, I> {
        PushIterFuture {
            owner: self,
            iter: Some(iter.peekable()),
        }
    }

    /// Wait for the buffer to have at least `count` free places for items or to close.
    ///
    /// Panics if `count` is greater than buffer capacity.
    fn wait_vacant(&self, count: usize) -> WaitVacantFuture<'_, Self> {
        debug_assert!(count <= self.capacity().get());
        WaitVacantFuture {
            owner: self,
            count,
            done: false,
        }
    }

    /// Copy slice contents to the buffer waiting asynchronously if the buffer is full.
    ///
    /// Future returns:
    /// + `Ok` - all slice contents are copied.
    /// + `Err(count)` - the corresponding consumer was dropped, number of copied items returned.
    fn push_slice_all<'a: 'b, 'b>(&'a mut self, slice: &'b [Self::Item]) -> PushSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }
}

pub struct PushFuture<'a, A: AsyncProducer> {
    owner: &'a mut A,
    item: Option<A::Item>,
}
impl<'a, A: AsyncProducer> Unpin for PushFuture<'a, A> {}
impl<'a, A: AsyncProducer> FusedFuture for PushFuture<'a, A> {
    fn is_terminated(&self) -> bool {
        self.item.is_none()
    }
}
impl<'a, A: AsyncProducer> Future for PushFuture<'a, A> {
    type Output = Result<(), A::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let item = self.item.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(Err(item));
            } else {
                match self.owner.try_push(item) {
                    Err(item) => {
                        self.item.replace(item);
                        self.owner.register_read_waker(cx.waker());
                        if self.owner.is_full() && !self.owner.is_closed() {
                            break Poll::Pending;
                        }
                    }
                    Ok(()) => break Poll::Ready(Ok(())),
                }
            }
        }
    }
}

pub struct PushSliceFuture<'a, 'b, A: AsyncProducer>
where
    A::Item: Copy,
{
    owner: &'a mut A,
    slice: Option<&'b [A::Item]>,
    count: usize,
}
impl<'a, 'b, A: AsyncProducer> Unpin for PushSliceFuture<'a, 'b, A> where A::Item: Copy {}
impl<'a, 'b, A: AsyncProducer> FusedFuture for PushSliceFuture<'a, 'b, A>
where
    A::Item: Copy,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<'a, 'b, A: AsyncProducer> Future for PushSliceFuture<'a, 'b, A>
where
    A::Item: Copy,
{
    type Output = Result<(), usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut slice = self.slice.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(Err(self.count));
            } else {
                let len = self.owner.push_slice(slice);
                slice = &slice[len..];
                self.count += len;
                if slice.is_empty() {
                    break Poll::Ready(Ok(()));
                } else {
                    self.slice.replace(slice);
                    self.owner.register_read_waker(cx.waker());
                    if self.owner.is_full() && !self.owner.is_closed() {
                        // should be full here, because `push_slice_all`
                        // has not been able to push all of the slice.
                        break Poll::Pending;
                    }
                }
            }
        }
    }
}

pub struct PushIterFuture<'a, A: AsyncProducer, I: Iterator<Item = A::Item>> {
    owner: &'a mut A,
    iter: Option<Peekable<I>>,
}
impl<'a, A: AsyncProducer, I: Iterator<Item = A::Item>> Unpin for PushIterFuture<'a, A, I> {}
impl<'a, A: AsyncProducer, I: Iterator<Item = A::Item>> FusedFuture for PushIterFuture<'a, A, I> {
    fn is_terminated(&self) -> bool {
        self.iter.is_none() || self.owner.is_closed()
    }
}
impl<'a, A: AsyncProducer, I: Iterator<Item = A::Item>> Future for PushIterFuture<'a, A, I> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut iter = self.iter.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(false);
            } else {
                self.owner.push_iter(&mut iter);
                if iter.peek().is_none() {
                    break Poll::Ready(true);
                } else {
                    self.iter.replace(iter);
                    self.owner.register_read_waker(cx.waker());
                    if self.owner.is_full() && !self.owner.is_closed() {
                        // should be full here, because `push_iter_all`
                        // has not been able to push all of the iterator.
                        break Poll::Pending;
                    }
                }
            }
        }
    }
}

pub struct WaitVacantFuture<'a, A: AsyncProducer> {
    owner: &'a A,
    count: usize,
    done: bool,
}
impl<'a, A: AsyncProducer> Unpin for WaitVacantFuture<'a, A> {}
impl<'a, A: AsyncProducer> FusedFuture for WaitVacantFuture<'a, A> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<'a, A: AsyncProducer> Future for WaitVacantFuture<'a, A> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            assert!(!self.done);
            let closed = self.owner.is_closed();
            if self.count <= self.owner.vacant_len() || closed {
                break Poll::Ready(());
            } else {
                self.owner.register_read_waker(cx.waker());
                if self.count > self.owner.vacant_len() && !self.owner.is_closed() {
                    break Poll::Pending;
                }
            }
        }
    }
}

impl<R: RbRef> Sink<<R::Target as Observer>::Item> for AsyncProd<R>
where
    R::Target: AsyncRingBuffer,
{
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            if self.is_closed() {
                break Poll::Ready(Err(()));
            } else if !self.is_full() {
                break Poll::Ready(Ok(()));
            } else {
                self.register_read_waker(cx.waker());
                if self.is_full() && !self.is_closed() {
                    // `poll_ready` is only valid if there is only a single producer.
                    // Otherwise `Ready(Ok())` doesn't mean anything, since vacant
                    // can be filled up by another producer, after future returns.
                    break Poll::Pending;
                }
            }
        }
    }
    fn start_send(mut self: Pin<&mut Self>, item: <R::Target as Observer>::Item) -> Result<(), Self::Error> {
        assert!(self.try_push(item).is_ok());
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.close();
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> AsyncWrite for AsyncProd<R>
where
    R::Target: AsyncRingBuffer<Item = u8>,
{
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        loop {
            if self.is_closed() {
                break Poll::Ready(Ok(0));
            } else {
                let count = self.push_slice(buf);
                if count == 0 {
                    self.register_read_waker(cx.waker());
                    if self.is_full() && !self.is_closed() {
                        break Poll::Pending;
                    }
                } else {
                    break Poll::Ready(Ok(count));
                }
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.close();
        Poll::Ready(Ok(()))
    }
}
