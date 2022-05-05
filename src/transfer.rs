use crate::{
    consumer::{GlobalConsumer, LocalConsumer},
    producer::{GlobalProducer, LocalProducer},
    ring_buffer::{Container, RingBufferRef},
};

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn transfer_local<'a, 'b, T, Cs, Cd>(
    src: &mut LocalConsumer<'a, T, Cs>,
    dst: &mut LocalProducer<'b, T, Cd>,
    count: Option<usize>,
) -> usize
where
    Cs: Container<T>,
    Cd: Container<T>,
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

pub fn transfer<T, Cs, Cd, Rs, Rd>(
    src: &mut GlobalConsumer<T, Cs, Rs>,
    dst: &mut GlobalProducer<T, Cd, Rd>,
    count: Option<usize>,
) -> usize
where
    Cs: Container<T>,
    Cd: Container<T>,
    Rs: RingBufferRef<T, Cs>,
    Rd: RingBufferRef<T, Cd>,
{
    transfer_local(&mut src.acquire(), &mut dst.acquire(), count)
}
