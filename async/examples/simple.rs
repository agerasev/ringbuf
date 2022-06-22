use async_ringbuf::AsyncHeapRb;
use futures::{executor::block_on, join};

async fn async_main() {
    let rb = AsyncHeapRb::<i32>::new(2);
    let (mut prod, mut cons) = rb.split();

    join!(
        async move {
            for i in 0..2 {
                prod.push(i).await;
            }
        },
        async move {
            for i in 0..2 {
                assert_eq!(cons.pop().await, i);
            }
        },
    );
}

fn main() {
    block_on(async_main());
}
