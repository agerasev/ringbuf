#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::mem::{self, MaybeUninit};

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

pub unsafe fn write_uninit_slice<'a, T: Copy>(dst: &'a mut [T], src: &[MaybeUninit<T>]) -> &'a mut [T] {
    dst.copy_from_slice(slice_assume_init_ref(src));
    dst
}

#[cfg(feature = "alloc")]
pub fn vec_to_uninit<T>(value: Vec<T>) -> Vec<MaybeUninit<T>> {
    let value = mem::ManuallyDrop::new(value);
    let ptr = &value as *const _ as *const Vec<MaybeUninit<T>>;
    unsafe { ptr.read() }
}

#[cfg(feature = "alloc")]
pub fn boxed_slice_to_uninit<T>(value: Box<[T]>) -> Box<[MaybeUninit<T>]> {
    let value = mem::ManuallyDrop::new(value);
    let ptr = &value as *const _ as *const Box<[MaybeUninit<T>]>;
    unsafe { ptr.read() }
}
