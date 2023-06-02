#![no_std]

use once_mut::once_mut;
use ringbuf::{traits::*, StaticRb};

once_mut! {
    static mut RB: StaticRb::<i32, 1> = StaticRb::default();
}

fn main() {
    let (mut prod, mut cons) = RB.take().unwrap().split_ref();

    assert_eq!(prod.try_push(123), Ok(()));
    assert_eq!(prod.try_push(321), Err(321));

    assert_eq!(cons.try_pop(), Some(123));
    assert_eq!(cons.try_pop(), None);
}
