#![no_std]

use ringbuf::StaticRb;

fn main() {
    const RB_SIZE: usize = 1;
    let mut rb = StaticRb::<i32, RB_SIZE>::default();
    let (mut prod, mut cons) = rb.split_ref();

    assert_eq!(prod.push(123), Ok(()));
    assert_eq!(prod.push(321), Err(321));

    assert_eq!(cons.pop(), Some(123));
    assert_eq!(cons.pop(), None);
}
