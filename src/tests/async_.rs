use crate::AsyncHeapRingBuffer;
use futures::{
    executor::{block_on, ThreadPool},
    join,
    task::{noop_waker_ref, AtomicWaker},
};

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

#[test]
fn push_pop() {
    let attempts = 256;
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<i32>::new(2).split_async();
    block_on(async {
        join!(
            async {
                for i in 0..attempts {
                    prod.push(i).await;
                }
            },
            async {
                for i in 0..attempts {
                    assert_eq!(cons.pop().await, i);
                }
            }
        );
    });
}

#[test]
fn push_pop_pool() {
    use std::{println, thread};

    let attempts = 1024;
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<i32>::new(2).split_async();
    let pool = ThreadPool::builder().pool_size(2).create().unwrap();
    pool.spawn_ok(async move {
        for i in 0..attempts {
            println!("Push thread {:?}", thread::current().id());
            prod.push(i).await;
        }
    });
    pool.spawn_ok(async move {
        for i in 0..attempts {
            println!("Pop thread {:?}", thread::current().id());
            assert_eq!(cons.pop().await, i);
        }
    });
    thread::sleep_ms(1000);
}
