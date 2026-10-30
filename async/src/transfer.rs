use crate::{consumer::AsyncConsumer, producer::AsyncProducer};
use core::{future::poll_fn, task::Poll};

/// Transfer items from one ring buffer to another.
///
/// Returns the number of items delivered. `Some(n)` transfers at most `n` items;
/// `None` transfers until the source is drained and closed or the destination is closed.
///
/// # Cancel safety
///
/// Cancelling preserves completed transfers and leaves all undelivered items in
/// the source. No item is held outside the queues while waiting for progress.
pub async fn async_transfer<T, As: AsyncConsumer<Item = T>, Ad: AsyncProducer<Item = T>>(
    src: &mut As,
    dst: &mut Ad,
    count: Option<usize>,
) -> usize {
    let mut actual_count = 0;
    poll_fn(|cx| {
        // Register both sides, then recheck to avoid missing progress or closure
        // between observing the queues and installing their wakers.
        for pass in 0..2 {
            if count == Some(actual_count) || dst.is_closed() {
                return Poll::Ready(actual_count);
            }
            let closed = src.is_closed();
            let moved = ringbuf::transfer(src, dst, count.map(|n| n - actual_count));
            actual_count += moved;
            if count == Some(actual_count) || (closed && src.is_empty()) {
                return Poll::Ready(actual_count);
            }
            if pass == 0 {
                src.register_waker(cx.waker());
                dst.register_waker(cx.waker());
            } else if moved > 0 {
                // Bound work per poll even when the peers keep both queues ready.
                cx.waker().wake_by_ref();
            }
        }
        Poll::Pending
    })
    .await
}
