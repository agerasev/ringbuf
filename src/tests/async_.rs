use crate::AsyncHeapRingBuffer;
use futures::task::{noop_waker_ref, AtomicWaker};
use std::{sync::mpsc, vec, vec::Vec};

#[test]
fn atomic_waker() {
    let waker = AtomicWaker::new();
    assert!(waker.take().is_none());

    waker.register(noop_waker_ref());
    assert!(waker.take().is_some());
    assert!(waker.take().is_none());

    waker.register(noop_waker_ref());
    waker.wake();
    assert!(waker.take().is_none());
}

macro_rules! execute {
    ( $( $tasks:expr ),* $(,)? ) => {
        futures::executor::block_on(async {
            futures::join!($($tasks),*)
        });
    };
}

macro_rules! execute_concurrently {
    ( $pool_size:expr, $( $tasks:expr ),* $(,)? ) => {{
        let pool = futures::executor::ThreadPool::builder()
            .pool_size(2)
            .create()
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let mut cnt = 0;
        $(
            let ltx = tx.clone();
            pool.spawn_ok(async move {
                $tasks.await;
                ltx.send(()).unwrap();
            });
            cnt += 1;
        )*
        for _ in 0..cnt {
            rx.recv().unwrap();
        }
    }};
}

#[test]
fn push_pop() {
    let attempts = 1024;
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<i32>::new(2).split_async();
    execute_concurrently!(
        2,
        async move {
            for i in 0..attempts {
                prod.push(i).await;
            }
        },
        async move {
            for i in 0..attempts {
                assert_eq!(cons.pop().await, i);
            }
        },
    );
}

#[test]
fn push_pop_slice() {
    let attempts = 1024;
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<usize>::new(3).split_async();
    execute_concurrently!(
        2,
        async move {
            let data = (0..attempts).collect::<Vec<_>>();
            prod.push_slice(&data).await;
        },
        async move {
            let mut data = vec![0; attempts];
            cons.pop_slice(&mut data).await;
            assert!(data.into_iter().eq(0..attempts));
        },
    );
}
