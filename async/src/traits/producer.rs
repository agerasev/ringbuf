use core::{
    future::Future,
    iter::Peekable,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures_util::future::FusedFuture;
use ringbuf::traits::Producer;
#[cfg(feature = "std")]
use std::io;

pub trait AsyncProducer: Producer {
    fn register_waker(&self, waker: &Waker);

    fn close(&mut self);
    /// Whether the corresponding consumer was closed.
    fn is_closed(&self) -> bool {
        !self.read_is_held()
    }

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
    /// In debug mode panics if `count` is greater than buffer capacity.
    ///
    /// The method takes `&mut self` because only single [`WaitVacantFuture`] is allowed at a time.
    fn wait_vacant(&mut self, count: usize) -> WaitVacantFuture<'_, Self> {
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
    fn push_exact<'a: 'b, 'b>(&'a mut self, slice: &'b [Self::Item]) -> PushSliceFuture<'a, 'b, Self>
    where
        Self::Item: Copy,
    {
        PushSliceFuture {
            owner: self,
            slice: Some(slice),
            count: 0,
        }
    }

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        let mut waker_registered = false;
        loop {
            if self.is_closed() {
                break Poll::Ready(false);
            }
            if !self.is_full() {
                break Poll::Ready(true);
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.register_waker(cx.waker());
            waker_registered = true;
        }
    }

    #[cfg(feature = "std")]
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>>
    where
        Self: AsyncProducer<Item = u8> + Unpin,
    {
        let mut waker_registered = false;
        loop {
            if self.is_closed() {
                break Poll::Ready(Ok(0));
            }
            let count = self.push_slice(buf);
            if count > 0 {
                break Poll::Ready(Ok(count));
            }
            if waker_registered {
                break Poll::Pending;
            }
            self.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

pub struct PushFuture<'a, A: AsyncProducer + ?Sized> {
    owner: &'a mut A,
    item: Option<A::Item>,
}
impl<A: AsyncProducer> Unpin for PushFuture<'_, A> {}
impl<A: AsyncProducer> FusedFuture for PushFuture<'_, A> {
    fn is_terminated(&self) -> bool {
        self.item.is_none()
    }
}
impl<A: AsyncProducer> Future for PushFuture<'_, A> {
    type Output = Result<(), A::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let item = self.item.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(Err(item));
            }
            let push_result = self.owner.try_push(item);
            if push_result.is_ok() {
                break Poll::Ready(Ok(()));
            }
            self.item.replace(push_result.unwrap_err());
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

pub struct PushSliceFuture<'a, 'b, A: AsyncProducer + ?Sized>
where
    A::Item: Copy,
{
    owner: &'a mut A,
    slice: Option<&'b [A::Item]>,
    count: usize,
}
impl<A: AsyncProducer> Unpin for PushSliceFuture<'_, '_, A> where A::Item: Copy {}
impl<A: AsyncProducer> FusedFuture for PushSliceFuture<'_, '_, A>
where
    A::Item: Copy,
{
    fn is_terminated(&self) -> bool {
        self.slice.is_none()
    }
}
impl<A: AsyncProducer> Future for PushSliceFuture<'_, '_, A>
where
    A::Item: Copy,
{
    type Output = Result<(), usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let mut slice = self.slice.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(Err(self.count));
            }
            let len = self.owner.push_slice(slice);
            slice = &slice[len..];
            self.count += len;
            if slice.is_empty() {
                break Poll::Ready(Ok(()));
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

pub struct PushIterFuture<'a, A: AsyncProducer + ?Sized, I: Iterator<Item = A::Item>> {
    owner: &'a mut A,
    iter: Option<Peekable<I>>,
}
impl<A: AsyncProducer, I: Iterator<Item = A::Item>> Unpin for PushIterFuture<'_, A, I> {}
impl<A: AsyncProducer, I: Iterator<Item = A::Item>> FusedFuture for PushIterFuture<'_, A, I> {
    fn is_terminated(&self) -> bool {
        self.iter.is_none() || self.owner.is_closed()
    }
}
impl<A: AsyncProducer, I: Iterator<Item = A::Item>> Future for PushIterFuture<'_, A, I> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            let mut iter = self.iter.take().unwrap();
            if self.owner.is_closed() {
                break Poll::Ready(false);
            }
            self.owner.push_iter(&mut iter);
            if iter.peek().is_none() {
                break Poll::Ready(true);
            }
            self.iter.replace(iter);
            if waker_registered {
                break Poll::Pending;
            }
            self.owner.register_waker(cx.waker());
            waker_registered = true;
        }
    }
}

pub struct WaitVacantFuture<'a, A: AsyncProducer + ?Sized> {
    owner: &'a A,
    count: usize,
    done: bool,
}
impl<A: AsyncProducer> Unpin for WaitVacantFuture<'_, A> {}
impl<A: AsyncProducer> FusedFuture for WaitVacantFuture<'_, A> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<A: AsyncProducer> Future for WaitVacantFuture<'_, A> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waker_registered = false;
        loop {
            assert!(!self.done);
            let closed = self.owner.is_closed();
            if self.count <= self.owner.vacant_len() || closed {
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
