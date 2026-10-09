use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures_util::future::FusedFuture;
use ringbuf::error::{TransferError, WaitError};
use ringbuf::traits::Consumer;
#[cfg(feature = "std")]
use std::io;

pub trait AsyncConsumer: ringbuf::traits::Presence + Consumer {
    /// Compatibility spelling for the streaming `pop_all` operation.
    fn pop_exact<'a: 'b, 'b>(&'a mut self, slice: &'b mut [Self::Item]) -> PopSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        self.pop_all(slice)
    }

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
    ///
    /// # Cancel safety
    ///
    /// If future is cancelled then no item removed from the ring buffer.
    fn pop(&mut self) -> PopFuture<'_, Self> {
        PopFuture { owner: self, done: false }
    }

    /// Wait for the buffer to contain at least `count` items or to close.
    ///
    /// Returns `TooLarge` immediately if `count` exceeds capacity.
    ///
    /// The method takes `&mut self` because only single [`WaitOccupiedFuture`] is allowed at a time.
    ///
    /// # Cancel safety
    ///
    /// The future can be safely cancelled.
    fn wait_occupied(&mut self, count: usize) -> WaitOccupiedFuture<'_, Self> {
        unsafe { self.prepare_read() };
        WaitOccupiedFuture {
            owner: self,
            count,
            done: false,
        }
    }

    /// Fill slice with items from the ring buffer waiting asynchronously until slice filled or corresponding producer closed.
    ///
    /// Future returns:
    /// + `Ok` - the whole slice is filled with the items from the buffer.
    /// + `Err(TransferError)` - the buffer is empty and the corresponding producer was dropped, number of items copied to slice is returned.
    ///
    /// # Cancel safety
    ///
    /// If future is cancelled then slice can be partially filled.
    /// The number of items already copied can be examined by [`PopSliceFuture::count`].
    fn pop_all<'a: 'b, 'b>(&'a mut self, slice: &'b mut [Self::Item]) -> PopSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        PopSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }

    /// Fill `vec` with items from the ring buffer waiting asynchronously until corresponding producer closed.
    ///
    /// # Cancel safety
    ///
    /// If future is cancelled then `vec` contains items taken from RB before cancellation.
    #[cfg(feature = "alloc")]
    fn pop_until_end<'a: 'b, 'b>(&'a mut self, vec: &'b mut alloc::vec::Vec<Self::Item>) -> PopVecFuture<'a, 'b, Self> {
        self.pop_into_vec(vec, usize::MAX)
    }

    /// Append at most `limit` items, stopping normally on producer closure.
    /// Allocation failure is returned, with the collected prefix left in `vec`.
    #[cfg(feature = "alloc")]
    fn pop_into_vec<'a: 'b, 'b>(&'a mut self, vec: &'b mut alloc::vec::Vec<Self::Item>, limit: usize) -> PopVecFuture<'a, 'b, Self> {
        PopVecFuture {
            owner: self,
            vec: Some(vec),
            limit,
            count: 0,
        }
    }

    /// Poll for the next item in the ring buffer.
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

    /// Poll reading bytes from byte buffer.
    #[cfg(feature = "std")]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>
    where
        Self: AsyncConsumer<Item = u8> + Unpin,
    {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
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

/// # Cancel safety
///
/// If future is cancelled then no item removed from the ring buffer.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct PopFuture<'a, A: AsyncConsumer + ?Sized> {
    owner: &'a mut A,
    done: bool,
}
impl<A: AsyncConsumer> Unpin for PopFuture<'_, A> {}
impl<A: AsyncConsumer> FusedFuture for PopFuture<'_, A> {
    fn is_terminated(&self) -> bool {
        self.done
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
                self.done = true;
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

/// # Cancel safety
///
/// If future is cancelled then slice can be partially filled.
#[must_use = "futures do nothing unless you `.await` or poll them"]
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
    type Output = Result<usize, TransferError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let closed = self.owner.is_closed();
            let mut slice = self.slice.take().unwrap();
            let len = self.owner.pop_slice(slice);
            slice = &mut slice[len..];
            self.count += len;
            if slice.is_empty() {
                break Poll::Ready(Ok(self.count));
            }
            if closed {
                break Poll::Ready(Err(TransferError {
                    completed: self.count,
                    reason: WaitError::Closed,
                }));
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
impl<A: AsyncConsumer> PopSliceFuture<'_, '_, A>
where
    A::Item: Copy,
{
    /// Number of items already copied from the ring buufer to the slice provided.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// # Cancel safety
///
/// If future is cancelled then `vec` contains items taken from RB before cancellation.
#[cfg(feature = "alloc")]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct PopVecFuture<'a, 'b, A: AsyncConsumer + ?Sized> {
    owner: &'a mut A,
    vec: Option<&'b mut alloc::vec::Vec<A::Item>>,
    limit: usize,
    count: usize,
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
    type Output = Result<usize, ringbuf::error::CollectError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let closed = self.owner.is_closed();
            let vec = self.vec.take().unwrap();

            loop {
                if self.count == self.limit {
                    return Poll::Ready(Ok(self.count));
                }
                if vec.len() == vec.capacity() {
                    if let Err(error) = vec.try_reserve((self.limit - self.count).min(vec.capacity().max(16))) {
                        return Poll::Ready(Err(ringbuf::error::CollectError::Allocation {
                            completed: self.count,
                            error,
                        }));
                    }
                }
                let take = (self.limit - self.count).min(vec.spare_capacity_mut().len());
                let n = self.owner.pop_slice_uninit(&mut vec.spare_capacity_mut()[..take]);
                self.count += n;
                if n == 0 {
                    break;
                }
                unsafe { vec.set_len(vec.len() + n) };
            }

            if closed {
                break Poll::Ready(Ok(self.count));
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

/// # Cancel safety
///
/// The future can be safely cancelled.
#[must_use = "futures do nothing unless you `.await` or poll them"]
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
    type Output = Result<(), WaitError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            if self.done {
                return Poll::Pending;
            }
            let closed = self.owner.is_closed();
            if self.count > self.owner.capacity().get() {
                self.done = true;
                break Poll::Ready(Err(WaitError::TooLarge {
                    requested: self.count,
                    capacity: self.owner.capacity().get(),
                }));
            }
            if self.count <= self.owner.occupied_len() {
                self.done = true;
                break Poll::Ready(Ok(()));
            }
            if closed {
                self.done = true;
                break Poll::Ready(Err(WaitError::Closed));
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

#[allow(unused_imports)]
use ringbuf::traits::Presence;

#[cfg(feature = "alloc")]
impl<A: AsyncConsumer> PopVecFuture<'_, '_, A> {
    pub fn count(&self) -> usize {
        self.count
    }
}
