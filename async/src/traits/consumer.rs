use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures::future::FusedFuture;
use ringbuf::traits::Consumer;
#[cfg(feature = "std")]
use std::io;

pub trait AsyncConsumer: Consumer {
    fn register_waker(&self, waker: &Waker);

    fn close(&mut self);
    /// Whether the corresponding producer was closed.
    fn is_closed(&self) -> bool {
        !self.write_is_held()
    }

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
    /// In debug mode panics if `count` is greater than buffer capacity.
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>
    where
        Self: Unpin,
    {
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

    #[cfg(feature = "std")]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>
    where
        Self: AsyncConsumer<Item = u8> + Unpin,
    {
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
            if let Some(item) = self.owner.try_pop() {
                self.done = true;
                break Poll::Ready(Some(item));
            }
            if closed {
                break Poll::Ready(None);
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
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
            }
            if closed {
                break Poll::Ready(Err(self.count));
            }
            self.slice.replace(slice);
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
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
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}
