#![allow(deprecated)]
use crate::{ArrayRb, traits::*};
#[test]
fn compatibility_name_uses_deferred_publication() {
    let mut rb = ArrayRb::<i32, 2>::default();
    let (p, mut c) = rb.split_ref();
    let mut p = p.freeze();
    p.try_push(1).unwrap();
    assert!(c.try_pop().is_none());
    p.commit();
    assert_eq!(c.try_pop(), Some(1));
}
