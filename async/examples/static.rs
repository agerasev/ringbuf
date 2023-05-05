#![no_std]
use async_ringbuf::AsyncRb;
use futures::{executor::block_on, join};
use ringbuf::StaticRb;

async fn async_main() {
    const RB_SIZE: usize = 1;
    let mut rb = AsyncRb::from_base(StaticRb::<i32, RB_SIZE>::default());
    let (prod, cons) = rb.split_ref();

    join!(
        async move {
            let mut prod = prod;
            prod.push(123).await.unwrap();
            prod.push(321).await.unwrap();
        },
        async move {
            let mut cons = cons;
            assert_eq!(cons.pop().await, Some(123));
            assert_eq!(cons.pop().await, Some(321));
            assert_eq!(cons.pop().await, None);
        },
    );
}

fn main() {
    block_on(async_main());
}
