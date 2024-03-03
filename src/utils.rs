#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::{
    mem::{self, MaybeUninit},
    ptr,
};

// TODO: Remove on `maybe_uninit_uninit_array` stabilization.
pub const fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
}

// TODO: Remove on `maybe_uninit_slice` stabilization.
pub unsafe fn slice_as_uninit_mut<T>(slice: &mut [T]) -> &mut [MaybeUninit<T>] {
    &mut *(slice as *mut [T] as *mut [MaybeUninit<T>])
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

pub fn move_uninit_slice<T>(dst: &mut [MaybeUninit<T>], src: &[MaybeUninit<T>]) {
    assert_eq!(dst.len(), src.len());
    for i in 0..dst.len() {
        unsafe { *dst.get_unchecked_mut(i) = ptr::read(src.get_unchecked(i) as *const _) };
    }
}

pub fn array_to_uninit<T, const N: usize>(value: [T; N]) -> [MaybeUninit<T>; N] {
    let value = MaybeUninit::new(value);
    let this = &value as *const _ as *const [MaybeUninit<T>; N];
    unsafe { this.read() }
}

#[cfg(feature = "alloc")]
pub fn vec_to_uninit<T>(mut value: Vec<T>) -> Vec<MaybeUninit<T>> {
    let (ptr, len, cap) = (value.as_mut_ptr() as *mut MaybeUninit<T>, value.len(), value.capacity());
    mem::forget(value);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

#[cfg(feature = "alloc")]
pub fn boxed_slice_to_uninit<T>(value: Box<[T]>) -> Box<[MaybeUninit<T>]> {
    let ptr = Box::into_raw(value) as *mut [MaybeUninit<T>];
    unsafe { Box::from_raw(ptr) }
}
