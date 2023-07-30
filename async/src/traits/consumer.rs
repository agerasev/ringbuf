use super::{AsyncObserver, AsyncRingBuffer};
use crate::halves::AsyncCons;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
#[cfg(feature = "std")]
use futures::io::AsyncRead;
use futures::{future::FusedFuture, Stream};
use ringbuf::{
    rb::traits::RbRef,
    traits::{Consumer, Observer},
};
#[cfg(feature = "std")]
use std::io;

pub trait AsyncConsumer: AsyncObserver + Consumer {
    fn register_write_waker(&self, waker: &Waker);

    /// Pop item from the ring buffer waiting asynchronously if the buffer is empty.
    ///
    /// Future returns:
    /// + `Some(item)` - an item is taken.
    /// + `None` - the buffer is empty and the corresponding producer was dropped.
    fn pop(&mut self) -> PopFuture<'_, Self> {
        PopFuture { owner: self, done: false }
    }

    /// Wait for the buffer to contain at least `count` items or to close.
    ///
    /// Panics if `count` is greater than buffer capacity.
    fn wait_occupied(&mut self, count: usize) -> WaitOccupiedFuture<'_, Self> {
        debug_assert!(count <= self.capacity().get());
        WaitOccupiedFuture {
            owner: self,
            count,
            done: false,
        }
    }

    /// Pop item from the ring buffer waiting asynchronously if the buffer is empty.
    ///
    /// Future returns:
    /// + `Ok` - the whole slice is filled with the items from the buffer.
    /// + `Err(count)` - the buffer is empty and the corresponding producer was dropped, number of items copied to slice is returned.
    fn pop_slice_all<'a: 'b, 'b>(&'a mut self, slice: &'b mut [Self::Item]) -> PopSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        PopSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }
}

pub struct PopFuture<'a, A: AsyncConsumer> {
    owner: &'a mut A,
    done: bool,
}
impl<'a, A: AsyncConsumer> Unpin for PopFuture<'a, A> {}
impl<'a, A: AsyncConsumer> FusedFuture for PopFuture<'a, A> {
    fn is_terminated(&self) -> bool {
        self.done || self.owner.is_closed()
    }
}
impl<'a, A: AsyncConsumer> Future for PopFuture<'a, A> {
    type Output = Option<A::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            assert!(!self.done);
            let closed = self.owner.is_closed();
            #[cfg(feature = "std")]
            std::println!("PopFuture::poll: closed={}", closed);
            match self.owner.try_pop() {
                Some(item) => {
                    self.done = true;
                    break Poll::Ready(Some(item));
                }
                None => {
                    if closed {
                        break Poll::Ready(None);
                    } else {
                        if waker_registered {
                            break Poll::Pending;
                        } else {
                            self.owner.register_write_waker(cx.waker());
                            waker_registered = true;
                        }
                    }
                }
            }
        }
    }
}

pub struct PopSliceFuture<'a, 'b, A: AsyncConsumer>
where
    A::Item: Copy,
{
    owner: &'a mut A,
    slice: Option<&'b mut [A::Item]>,
    count: usize,
}
impl<'a, 'b, A: AsyncConsumer> Unpin for PopSliceFuture<'a, 'b, A> where A::Item: Copy {}
impl<'a, 'b, A: AsyncConsumer> FusedFuture for PopSliceFuture<'a, 'b, A>
where
    A::Item: Copy,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<'a, 'b, A: AsyncConsumer> Future for PopSliceFuture<'a, 'b, A>
where
    A::Item: Copy,
{
    type Output = Result<(), usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let closed = self.owner.is_closed();
            let mut slice = self.slice.take().unwrap();
            let len = self.owner.pop_slice(slice);
            slice = &mut slice[len..];
            self.count += len;
            if slice.is_empty() {
                break Poll::Ready(Ok(()));
            } else if closed {
                break Poll::Ready(Err(self.count));
            } else {
                self.slice.replace(slice);
                if waker_registered {
                    break Poll::Pending;
                } else {
                    self.owner.register_write_waker(cx.waker());
                    waker_registered = true;
                }
            }
        }
    }
}

pub struct WaitOccupiedFuture<'a, A: AsyncConsumer> {
    owner: &'a A,
    count: usize,
    done: bool,
}
impl<'a, A: AsyncConsumer> Unpin for WaitOccupiedFuture<'a, A> {}
impl<'a, A: AsyncConsumer> FusedFuture for WaitOccupiedFuture<'a, A> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<'a, A: AsyncConsumer> Future for WaitOccupiedFuture<'a, A> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            assert!(!self.done);
            let closed = self.owner.is_closed();
            if self.count <= self.owner.occupied_len() || closed {
                break Poll::Ready(());
            } else {
                if waker_registered {
                    break Poll::Pending;
                } else {
                    self.owner.register_write_waker(cx.waker());
                    waker_registered = true;
                }
            }
        }
    }
}

impl<R: RbRef> Stream for AsyncCons<R>
where
    R::Target: AsyncRingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut waker_registered = false;
        loop {
            let closed = self.is_closed();
            match self.try_pop() {
                Some(item) => break Poll::Ready(Some(item)),
                None => {
                    if closed {
                        break Poll::Ready(None);
                    } else {
                        if waker_registered {
                            break Poll::Pending;
                        } else {
                            self.register_write_waker(cx.waker());
                            waker_registered = true;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> AsyncRead for AsyncCons<R>
where
    R::Target: AsyncRingBuffer<Item = u8>,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let mut waker_registered = false;
        loop {
            let closed = self.is_closed();
            let len = self.pop_slice(buf);
            if len != 0 || closed {
                break Poll::Ready(Ok(len));
            } else {
                if waker_registered {
                    break Poll::Pending;
                } else {
                    self.register_write_waker(cx.waker());
                    waker_registered = true;
                }
            }
        }
    }
}
