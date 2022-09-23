use crate::ring_buffer::{AsyncRbBase, AsyncRbWrite};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
#[cfg(feature = "std")]
use futures::io::AsyncWrite;
use futures::{future::FusedFuture, sink::Sink};
use ringbuf::{ring_buffer::RbRef, Producer};
#[cfg(feature = "std")]
use std::io;

pub struct AsyncProducer<T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    base: Producer<T, R>,
    /// Flag that marks that *producer* is in closed state.
    ///
    /// *Don't be confused with an atomic [`AsyncRb::closed`].*
    closed: bool,
}

impl<T, R: RbRef> AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    pub fn from_sync(base: Producer<T, R>) -> Self {
        Self {
            base,
            closed: false,
        }
    }
    pub fn as_sync(&self) -> &Producer<T, R> {
        &self.base
    }
    pub fn as_mut_sync(&mut self) -> &mut Producer<T, R> {
        &mut self.base
    }

    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }
    pub fn is_full(&self) -> bool {
        self.base.is_full()
    }
    pub fn len(&self) -> usize {
        self.base.len()
    }
    pub fn free_len(&self) -> usize {
        self.base.free_len()
    }

    /// Closes the producer. *All subsequent writes will panic.*
    pub fn close(&mut self) {
        unsafe { self.base.rb().close_tail() };
        self.closed = true;
    }
    /// Check if the corresponding consumer is dropped.
    pub fn is_closed(&self) -> bool {
        self.closed || self.base.rb().is_closed()
    }

    fn register_waker(&self, waker: &Waker) {
        unsafe { self.base.rb().register_head_waker(waker) };
    }

    /// Push item to the ring buffer waiting asynchronously if the buffer is full.
    ///
    /// Future returns:
    /// + `Ok` - item successfully pushed.
    /// + `Err(item)` - the corresponding consumer was dropped, item is returned back.
    pub fn push(&mut self, item: T) -> PushFuture<'_, T, R> {
        assert!(!self.closed);
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
    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: I) -> PushIterFuture<'_, T, R, I> {
        assert!(!self.closed);
        PushIterFuture {
            owner: self,
            iter: Some(iter),
        }
    }

    /// Wait for the buffer to have at least `free_len` free places for items or to close.
    ///
    /// Panics if `free_len` is greater than buffer capacity.
    pub fn wait_free(&self, free_len: usize) -> WaitFreeFuture<'_, T, R> {
        assert!(free_len <= self.capacity());
        WaitFreeFuture {
            owner: self,
            free_len,
            done: false,
        }
    }
}

impl<T: Copy, R: RbRef> AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    /// Copy slice contents to the buffer waiting asynchronously if the buffer is full.
    ///
    /// Future returns:
    /// + `Ok` - all slice contents are copied.
    /// + `Err(count)` - the corresponding consumer was dropped, number of copied items returned.
    pub fn push_slice<'a: 'b, 'b>(&'a mut self, slice: &'b [T]) -> PushSliceFuture<'a, 'b, T, R> {
        assert!(!self.closed);
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }
}

impl<T, R: RbRef> Drop for AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn drop(&mut self) {
        unsafe { self.base.rb().close_tail() };
    }
}

impl<T, R: RbRef> Unpin for AsyncProducer<T, R> where R::Rb: AsyncRbWrite<T> {}

pub struct PushFuture<'a, T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    item: Option<T>,
}
impl<'a, T, R: RbRef> Unpin for PushFuture<'a, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, T, R: RbRef> FusedFuture for PushFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.item.is_none()
    }
}
impl<'a, T, R: RbRef> Future for PushFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = Result<(), T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let item = self.item.take().unwrap();
        self.owner.register_waker(cx.waker());
        if self.owner.is_closed() {
            Poll::Ready(Err(item))
        } else {
            match self.owner.base.push(item) {
                Err(item) => {
                    self.item.replace(item);
                    Poll::Pending
                }
                Ok(()) => Poll::Ready(Ok(())),
            }
        }
    }
}

pub struct PushSliceFuture<'a, 'b, T: Copy, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    slice: Option<&'b [T]>,
    count: usize,
}
impl<'a, 'b, T: Copy, R: RbRef> Unpin for PushSliceFuture<'a, 'b, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, 'b, T: Copy, R: RbRef> FusedFuture for PushSliceFuture<'a, 'b, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<'a, 'b, T: Copy, R: RbRef> Future for PushSliceFuture<'a, 'b, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = Result<(), usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut slice = self.slice.take().unwrap();
        if self.owner.is_closed() {
            Poll::Ready(Err(self.count))
        } else {
            let len = self.owner.base.push_slice(slice);
            slice = &slice[len..];
            self.count += len;
            if slice.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                self.slice.replace(slice);
                Poll::Pending
            }
        }
    }
}

pub struct PushIterFuture<'a, T, R: RbRef, I: Iterator<Item = T>>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a mut AsyncProducer<T, R>,
    iter: Option<I>,
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> Unpin for PushIterFuture<'a, T, R, I> where
    R::Rb: AsyncRbWrite<T>
{
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> FusedFuture for PushIterFuture<'a, T, R, I>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.iter.is_none()
    }
}
impl<'a, T, R: RbRef, I: Iterator<Item = T>> Future for PushIterFuture<'a, T, R, I>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = Result<(), I>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.owner.register_waker(cx.waker());
        let mut iter = self.iter.take().unwrap();
        if self.owner.is_closed() {
            Poll::Ready(Err(iter))
        } else {
            let iter_ended = {
                let mut local = self.owner.base.postponed();
                local.push_iter(&mut iter);
                !local.is_full()
            };
            if iter_ended {
                Poll::Ready(Ok(()))
            } else {
                self.iter.replace(iter);
                Poll::Pending
            }
        }
    }
}

pub struct WaitFreeFuture<'a, T, R: RbRef>
where
    R::Rb: AsyncRbWrite<T>,
{
    owner: &'a AsyncProducer<T, R>,
    free_len: usize,
    done: bool,
}
impl<'a, T, R: RbRef> Unpin for WaitFreeFuture<'a, T, R> where R::Rb: AsyncRbWrite<T> {}
impl<'a, T, R: RbRef> FusedFuture for WaitFreeFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<'a, T, R: RbRef> Future for WaitFreeFuture<'a, T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(!self.done);
        self.owner.register_waker(cx.waker());
        let closed = self.owner.is_closed();
        if self.free_len <= self.owner.free_len() || closed {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl<T, R: RbRef> Sink<T> for AsyncProducer<T, R>
where
    R::Rb: AsyncRbWrite<T>,
{
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        assert!(!self.closed);
        self.register_waker(cx.waker());
        if self.is_closed() {
            Poll::Ready(Err(()))
        } else if self.base.is_full() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        assert!(!self.closed);
        assert!(self.base.push(item).is_ok());
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        assert!(!self.closed);
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        assert!(!self.closed);
        self.close();
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> AsyncWrite for AsyncProducer<u8, R>
where
    R::Rb: AsyncRbWrite<u8>,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        assert!(!self.closed);
        self.register_waker(cx.waker());
        if self.is_closed() {
            Poll::Ready(Ok(0))
        } else {
            let count = self.base.push_slice(buf);
            if count == 0 {
                Poll::Pending
            } else {
                Poll::Ready(Ok(count))
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        assert!(!self.closed);
        // Don't need to be flushed.
        Poll::Ready(Ok(()))
    }
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        assert!(!self.closed);
        self.close();
        Poll::Ready(Ok(()))
    }
}
