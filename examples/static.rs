#![no_std]

use ringbuf::{traits::*, StaticRb};

fn main() {
    const RB_SIZE: usize = 1;
    let rb = StaticRb::<i32, RB_SIZE>::default();
    let (mut prod, mut cons) = rb.split_ref();

    assert_eq!(prod.try_push(123), Ok(()));
    assert_eq!(prod.try_push(321), Err(321));

    assert_eq!(cons.try_pop(), Some(123));
    assert_eq!(cons.try_pop(), None);
}
