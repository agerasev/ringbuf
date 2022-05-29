use crate::{
    consumer::AsyncConsumer,
    producer::AsyncProducer,
    ring_buffer::{AsyncRbRead, AsyncRbWrite},
};
use ringbuf::ring_buffer::RbRef;

/// Tranfer data from one ring buffer to another.
///
/// `count` is the number of items being transfered.
///
/// If `count` is `Some(n)` then exactly `n` items will be transfered on completion
/// except the situation when the future is dropped before completion.
///
/// If `count` is `None` then transfer will be performed **indefinitely**.
/// The only way to stop it is to drop the future.  
pub async fn async_transfer<T, Rs: RbRef, Rd: RbRef>(
    src: &mut AsyncConsumer<T, Rs>,
    dst: &mut AsyncProducer<T, Rd>,
    mut count: Option<usize>,
) where
    Rs::Rb: AsyncRbRead<T>,
    Rd::Rb: AsyncRbWrite<T>,
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
