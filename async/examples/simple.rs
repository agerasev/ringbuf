use async_ringbuf::AsyncHeapRb;
use futures::{executor::block_on, join};

async fn async_main() {
    let rb = AsyncHeapRb::<i32>::new(2);
    let (prod, cons) = rb.split();

    join!(
        async move {
            let mut prod = prod;
            for i in 0..2 {
                prod.push(i).await.unwrap();
            }
        },
        async move {
            let mut cons = cons;
            for i in 0..2 {
                assert_eq!(cons.pop().await, Some(i));
            }
            assert_eq!(cons.pop().await, None);
        },
    );
}

fn main() {
    block_on(async_main());
}
