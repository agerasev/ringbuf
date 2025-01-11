#![no_std]

use lock_free_static::OnceMut;
use ringbuf::{traits::*, StaticRb};

static RB: OnceMut<StaticRb<i32, 1>> = OnceMut::new();

fn main() {
    RB.set(StaticRb::default()).ok().expect("RB already initialized");

    let (mut prod, mut cons) = RB.get_mut().expect("Mutable reference to RB already taken").split_ref();

    assert_eq!(prod.try_push(123), Ok(()));
    assert_eq!(prod.try_push(321), Err(321));

    assert_eq!(cons.try_pop(), Some(123));
    assert_eq!(cons.try_pop(), None);
}
