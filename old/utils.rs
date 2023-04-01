use core::{
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ops::Range,
};

/// Returns a pair of ranges between `head` and `tail` positions in a ring buffer with specific `capacity`.
///
/// `head` and `tail` may be arbitrary large, but must satisfy the following condition: `0 <= (head - tail) % (2 * capacity) <= capacity`.
/// Actual positions are taken modulo `capacity`.
///
/// The first range starts from `head`. If the first slice is empty then second slice is empty too.
pub fn ring_buffer_ranges(
    capacity: NonZeroUsize,
    head: usize,
    tail: usize,
) -> (Range<usize>, Range<usize>) {
    let (head_quo, head_rem) = (head / capacity, head % capacity);
    let (tail_quo, tail_rem) = (tail / capacity, tail % capacity);

    if (head_quo + tail_quo) % 2 == 0 {
        (head_rem..tail_rem, 0..0)
    } else {
        (head_rem..capacity.get(), 0..tail_rem)
    }
}

// TODO: Remove on `maybe_uninit_uninit_array` stabilization.
pub fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
}

// TODO: Remove on `maybe_uninit_slice` stabilization.
pub unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    &*(slice as *const [MaybeUninit<T>] as *const [T])
}

// TODO: Remove on `maybe_uninit_slice` stabilization.
pub unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    &mut *(slice as *mut [MaybeUninit<T>] as *mut [T])
}

// TODO: Remove on `maybe_uninit_write_slice` stabilization.
pub fn write_slice<'a, T: Copy>(dst: &'a mut [MaybeUninit<T>], src: &[T]) -> &'a mut [T] {
    let uninit_src: &[MaybeUninit<T>] = unsafe { mem::transmute(src) };
    dst.copy_from_slice(uninit_src);
    unsafe { slice_assume_init_mut(dst) }
}

pub unsafe fn write_uninit_slice<'a, T: Copy>(
    dst: &'a mut [T],
    src: &[MaybeUninit<T>],
) -> &'a mut [T] {
    dst.copy_from_slice(slice_assume_init_ref(src));
    dst
}
