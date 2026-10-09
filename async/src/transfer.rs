use crate::{consumer::AsyncConsumer, producer::AsyncProducer};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use ringbuf::error::{TransferError, WaitError};

/// Transfer directly between queues, with no item held outside either queue while
/// waiting. Cancellation preserves the completed prefix; inspect `count` first.
/// `None` drains until source closure; `Some(n)` requires all `n` items.
pub fn async_transfer<'a, T, C: AsyncConsumer<Item = T>, P: AsyncProducer<Item = T>>(
    src: &'a mut C,
    dst: &'a mut P,
    count: Option<usize>,
) -> TransferFuture<'a, C, P> {
    TransferFuture {
        src,
        dst,
        target: count,
        completed: 0,
        done: false,
    }
}

#[must_use = "futures must be polled or awaited"]
pub struct TransferFuture<'a, C: AsyncConsumer, P: AsyncProducer<Item = C::Item>> {
    src: &'a mut C,
    dst: &'a mut P,
    target: Option<usize>,
    completed: usize,
    done: bool,
}
impl<C: AsyncConsumer, P: AsyncProducer<Item = C::Item>> TransferFuture<'_, C, P> {
    pub fn count(&self) -> usize {
        self.completed
    }
}
impl<C: AsyncConsumer, P: AsyncProducer<Item = C::Item>> Unpin for TransferFuture<'_, C, P> {}
impl<C: AsyncConsumer, P: AsyncProducer<Item = C::Item>> futures_util::future::FusedFuture for TransferFuture<'_, C, P> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
impl<C: AsyncConsumer, P: AsyncProducer<Item = C::Item>> Future for TransferFuture<'_, C, P> {
    type Output = Result<usize, TransferError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.done {
            return Poll::Pending;
        }
        for pass in 0..2 {
            if this.target == Some(this.completed) {
                this.done = true;
                return Poll::Ready(Ok(this.completed));
            }
            if this.dst.is_closed() {
                this.done = true;
                return Poll::Ready(Err(TransferError {
                    completed: this.completed,
                    reason: WaitError::Closed,
                }));
            }
            let closed = this.src.is_closed();
            let moved = ringbuf::transfer(this.src, this.dst, this.target.map(|n| n - this.completed));
            this.completed += moved;
            if this.target == Some(this.completed) {
                this.done = true;
                return Poll::Ready(Ok(this.completed));
            }
            if closed && this.src.is_empty() {
                this.done = true;
                return Poll::Ready(if this.target.is_none() {
                    Ok(this.completed)
                } else {
                    Err(TransferError {
                        completed: this.completed,
                        reason: WaitError::Closed,
                    })
                });
            }
            if pass == 0 {
                this.src.register_waker(cx.waker());
                this.dst.register_waker(cx.waker());
            } else if moved > 0 {
                cx.waker().wake_by_ref();
            }
        }
        Poll::Pending
    }
}
