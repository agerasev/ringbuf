use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures_util::future::FusedFuture;
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
    ///
    /// The method takes `&mut self` because only single [`WaitOccupiedFuture`] is allowed at a time.
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
    fn pop_exact<'a: 'b, 'b>(&'a mut self, slice: &'b mut [Self::Item]) -> PopSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        PopSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }

    #[cfg(feature = "alloc")]
    fn pop_until_end<'a: 'b, 'b>(&'a mut self, vec: &'b mut alloc::vec::Vec<Self::Item>) -> PopVecFuture<'a, 'b, Self> {
        PopVecFuture {
            owner: self,
            vec: Some(vec),
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

pub struct PopFuture<'a, A: AsyncConsumer + ?Sized> {
    owner: &'a mut A,
    done: bool,
}
impl<A: AsyncConsumer> Unpin for PopFuture<'_, A> {}
impl<A: AsyncConsumer> FusedFuture for PopFuture<'_, A> {
    fn is_terminated(&self) -> bool {
        self.done || self.owner.is_closed()
    }
}
impl<A: AsyncConsumer> Future for PopFuture<'_, A> {
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

pub struct PopSliceFuture<'a, 'b, A: AsyncConsumer + ?Sized>
where
    A::Item: Copy,
{
    owner: &'a mut A,
    slice: Option<&'b mut [A::Item]>,
    count: usize,
}
impl<A: AsyncConsumer> Unpin for PopSliceFuture<'_, '_, A> where A::Item: Copy {}
impl<A: AsyncConsumer> FusedFuture for PopSliceFuture<'_, '_, A>
where
    A::Item: Copy,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<A: AsyncConsumer> Future for PopSliceFuture<'_, '_, A>
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

#[cfg(feature = "alloc")]
pub struct PopVecFuture<'a, 'b, A: AsyncConsumer + ?Sized> {
    owner: &'a mut A,
    vec: Option<&'b mut alloc::vec::Vec<A::Item>>,
}
#[cfg(feature = "alloc")]
impl<A: AsyncConsumer> Unpin for PopVecFuture<'_, '_, A> {}
#[cfg(feature = "alloc")]
impl<A: AsyncConsumer> FusedFuture for PopVecFuture<'_, '_, A> {
    fn is_terminated(&self) -> bool {
        self.vec.is_none()
    }
}
#[cfg(feature = "alloc")]
impl<A: AsyncConsumer> Future for PopVecFuture<'_, '_, A> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let closed = self.owner.is_closed();
            let vec = self.vec.take().unwrap();

            loop {
                if vec.len() == vec.capacity() {
                    vec.reserve(vec.capacity().max(16));
                }
                let n = self.owner.pop_slice_uninit(vec.spare_capacity_mut());
                if n == 0 {
                    break;
                }
                unsafe { vec.set_len(vec.len() + n) };
            }

            if closed {
                break Poll::Ready(());
            }
            self.vec.replace(vec);
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

pub struct WaitOccupiedFuture<'a, A: AsyncConsumer + ?Sized> {
    owner: &'a A,
    count: usize,
    done: bool,
}
impl<A: AsyncConsumer> Unpin for WaitOccupiedFuture<'_, A> {}
impl<A: AsyncConsumer> FusedFuture for WaitOccupiedFuture<'_, A> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<A: AsyncConsumer> Future for WaitOccupiedFuture<'_, A> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            assert!(!self.done);
            let closed = self.owner.is_closed();
            if self.count <= self.owner.occupied_len() || closed {
                self.done = true;
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
