#![no_std]
use async_ringbuf::AsyncRb;
use futures::{executor::block_on, join};
use ringbuf::StaticRb;

async fn async_main() {
    const RB_SIZE: usize = 1;
    let mut rb = AsyncRb::from_base(StaticRb::<i32, RB_SIZE>::default());
    let (mut prod, mut cons) = rb.split_ref();

    join!(
        async move {
            prod.push(123).await;
            prod.push(321).await;
        },
        async move {
            assert_eq!(cons.pop().await, 123);
            assert_eq!(cons.pop().await, 321);
        },
    );
}

fn main() {
    block_on(async_main());
}
