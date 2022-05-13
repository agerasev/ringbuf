use crate::{
    consumer::{Consumer, LocalConsumer},
    counter::Counter,
    producer::{LocalProducer, Producer},
    ring_buffer::RingBufferRef,
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

pub fn transfer<T, Rc, Rp>(
    src: &mut Consumer<T, Rc>,
    dst: &mut Producer<T, Rp>,
    count: Option<usize>,
) -> usize
where
    Rc: RingBufferRef<T>,
    Rp: RingBufferRef<T>,
{
    transfer_local(&mut src.acquire(), &mut dst.acquire(), count)
}
