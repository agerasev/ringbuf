use crate::{HeapRb, LocalRb, Rb};
use alloc::vec::Vec;
use core::mem::MaybeUninit;

#[test]
fn push() {
    let mut rb = HeapRb::<i32>::new(2);

    assert_eq!(rb.push_overwrite(0), None);
    assert_eq!(rb.push_overwrite(1), None);
    assert_eq!(rb.push_overwrite(2), Some(0));

    assert_eq!(rb.pop(), Some(1));
    assert_eq!(rb.pop(), Some(2));
    assert_eq!(rb.pop(), None);
}

#[test]
fn push_iter() {
    let mut rb = HeapRb::<i32>::new(2);
    rb.push_iter_overwrite([0, 1, 2, 3, 4, 5].into_iter());
    assert_eq!(rb.pop_iter().collect::<Vec<_>>(), [4, 5]);
}

#[test]
fn push_slice() {
    let mut rb = HeapRb::<i32>::new(2);
    rb.push_slice_overwrite(&[0, 1, 2, 3, 4, 5]);
    assert_eq!(rb.pop_iter().collect::<Vec<_>>(), [4, 5]);
}

#[test]
fn push_local() {
    let mut rb = LocalRb::<i32, [MaybeUninit<i32>; 2]>::default();

    assert_eq!(rb.push_overwrite(0), None);
    assert_eq!(rb.push_overwrite(1), None);
    assert_eq!(rb.push_overwrite(2), Some(0));

    assert_eq!(rb.pop(), Some(1));
    assert_eq!(rb.pop(), Some(2));
    assert_eq!(rb.pop(), None);
}
