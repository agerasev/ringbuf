use crate::{
    consumer::{AsyncConsumer, Consumer, LocalConsumer},
    counter::{AsyncCounter, Counter},
    producer::{AsyncProducer, LocalProducer, Producer},
    ring_buffer::{Container, RingBufferRef},
};

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// HeapConsumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn transfer_local<'a, 'b, T, Sc, Sp>(
    src: &mut LocalConsumer<'a, T, Sc>,
    dst: &mut LocalProducer<'b, T, Sp>,
    count: Option<usize>,
) -> usize
where
    Sc: Counter,
    Sp: Counter,
{
    let (src_left, src_right) = unsafe { src.as_uninit_slices() };
    let (dst_left, dst_right) = unsafe { dst.free_space_as_slices() };
    let src_iter = src_left.iter().chain(src_right.iter());
    let dst_iter = dst_left.iter_mut().chain(dst_right.iter_mut());

    let mut actual_count = 0;
    for (src_elem, dst_place) in src_iter.zip(dst_iter) {
        if let Some(count) = count {
            if actual_count >= count {
                break;
            }
        }
        unsafe { dst_place.write(src_elem.as_ptr().read()) };
        actual_count += 1;
    }
    unsafe { src.advance(actual_count) };
    unsafe { dst.advance(actual_count) };
    actual_count
}

pub fn transfer<T, Cc, Cp, Sc, Sp, Rc, Rp>(
    src: &mut Consumer<T, Cc, Sc, Rc>,
    dst: &mut Producer<T, Cp, Sp, Rp>,
    count: Option<usize>,
) -> usize
where
    Cc: Container<T>,
    Cp: Container<T>,
    Sc: Counter,
    Sp: Counter,
    Rc: RingBufferRef<T, Cc, Sc>,
    Rp: RingBufferRef<T, Cp, Sp>,
{
    transfer_local(&mut src.acquire(), &mut dst.acquire(), count)
}

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
