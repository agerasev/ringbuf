use crate::{consumer::AsyncConsumer, counter::AsyncCounter, producer::AsyncProducer};
use ringbuf::RingBufferRef;

/// Tranfer data from one ring buffer to another.
///
/// `count` is the number of items being transfered.
///
/// If `count` is `Some(n)` then exactly `n` items will be transfered on completion
/// except the situation when the future is dropped before completion.
///
/// If `count` is `None` then transfer will be performed **indefinitely**.
/// The only way to stop it is to drop the future.  
pub async fn async_transfer<T, Rc, Rp>(
    src: &mut AsyncConsumer<T, Rc>,
    dst: &mut AsyncProducer<T, Rp>,
    mut count: Option<usize>,
) where
    Rc: RingBufferRef<T, Counter = AsyncCounter>,
    Rp: RingBufferRef<T, Counter = AsyncCounter>,
{
    // TODO: Transfer multiple items at once.
    loop {
        if let Some(ref mut n) = count {
            if *n == 0 {
                break;
            }
            *n -= 1;
        }
        dst.push(src.pop().await).await;
    }
}
