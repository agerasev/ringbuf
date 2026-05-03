use async_ringbuf::{AsyncHeapRb, traits::*};
use futures::{executor::block_on, join};

async fn async_main() {
    let rb = AsyncHeapRb::<i32>::new(10);
    let (prod, cons) = rb.split();

    const N_ITERS: i32 = 100;

    join!(
        async move {
            let mut prod = prod;
            for i in 0..N_ITERS {
                prod.push(i).await.unwrap();
            }
        },
        async move {
            let mut cons = cons;
            for i in 0..N_ITERS {
                assert_eq!(cons.pop().await, Some(i));
            }
            assert_eq!(cons.pop().await, None);
        },
    );
}

fn main() {
    block_on(async_main());
}
