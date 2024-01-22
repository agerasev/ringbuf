use super::Rb;
use crate::{storage::Heap, traits::*};
use alloc::{boxed::Box, vec::Vec};

#[test]
fn new() {
    const CAP: usize = 2;
    let rb = Rb::<Heap<i32>>::new(CAP);
    let (mut prod, mut cons) = rb.split();

    assert_eq!(prod.capacity().get(), CAP);

    assert_eq!(prod.try_push(0), Ok(()));
    assert_eq!(prod.try_push(1), Ok(()));
    assert_eq!(prod.try_push(2), Err(2));

    assert_eq!(cons.try_pop(), Some(0));
    assert_eq!(cons.try_pop(), Some(1));
    assert_eq!(cons.try_pop(), None);
}

#[test]
fn from_vec() {
    let mut vec = Vec::with_capacity(2);
    vec.push(123);
    let rb = Rb::<Heap<i32>>::from(vec);
    let (mut prod, mut cons) = rb.split();

    assert_eq!(prod.capacity().get(), 2);
    assert_eq!(cons.occupied_len(), 1);

    assert_eq!(prod.try_push(321), Ok(()));
    assert_eq!(prod.try_push(444), Err(444));

    assert_eq!(cons.try_pop(), Some(123));
    assert_eq!(cons.try_pop(), Some(321));
    assert_eq!(cons.try_pop(), None);
}

#[test]
fn from_boxed_slice() {
    let boxed_slice = Box::new([123, 321]) as Box<[i32]>;
    let rb = Rb::<Heap<i32>>::from(boxed_slice);
    let (mut prod, mut cons) = rb.split();

    assert_eq!(prod.capacity().get(), 2);
    assert_eq!(cons.occupied_len(), 2);

    assert_eq!(prod.try_push(444), Err(444));

    assert_eq!(cons.try_pop(), Some(123));
    assert_eq!(cons.try_pop(), Some(321));
    assert_eq!(cons.try_pop(), None);
}
