use crate::{consumer::AsyncConsumer, counter::AsyncCounter, producer::AsyncProducer};
use ringbuf::{Container, RingBufferRef};

/// Tranfer data from one ring buffer to another.
///
/// `count` is the number of items being transfered.
///
/// If `count` is `Some(n)` then exactly `n` items will be transfered on completion
/// except the situation when the future is dropped before completion.
///
/// If `count` is `None` then transfer will be performed **indefinitely**.
/// The only way to stop it is to drop the future.  
pub async fn async_transfer<T, Cc, Cp, Rc, Rp>(
    src: &mut AsyncConsumer<T, Cc, Rc>,
    dst: &mut AsyncProducer<T, Cp, Rp>,
    mut count: Option<usize>,
) where
    Cc: Container<T>,
    Cp: Container<T>,
    Rc: RingBufferRef<T, Cc, AsyncCounter>,
    Rp: RingBufferRef<T, Cp, AsyncCounter>,
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
