use crate::{
    consumer::AsyncConsumer,
    producer::AsyncProducer,
    ring_buffer::{AsyncRbRead, AsyncRbWrite},
};
use ringbuf::ring_buffer::RbRef;

/// Tranfer data from one ring buffer to another.
///
/// `count` is the number of items to transfer.
/// The number of actually transfered items is returned.
///
/// If `count` is `Some(n)` then exactly `n` items will be transfered on completion
/// except the situation when `src` or `dst` is closed or the future is dropped before completion.
///
/// If `count` is `None` then transfer will be performed until the one or another ring buffer is closed.
/// Transfer also safely stopped if the future is dropped.
pub async fn async_transfer<T, Rs: RbRef, Rd: RbRef>(
    src: &mut AsyncConsumer<T, Rs>,
    dst: &mut AsyncProducer<T, Rd>,
    count: Option<usize>,
) -> usize
where
    Rs::Rb: AsyncRbRead<T>,
    Rd::Rb: AsyncRbWrite<T>,
{
    let mut actual_count = 0;
    // TODO: Transfer multiple items at once.
    loop {
        if count.as_ref().map_or(false, |n| actual_count == *n) {
            break;
        }
        actual_count += 1;

        match dst
            .push(match src.pop().await {
                Some(item) => item,
                None => break,
            })
            .await
        {
            Ok(()) => (),
            Err(_item) => break,
        };
    }
    actual_count
}
