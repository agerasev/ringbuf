use crate::AsyncHeapRingBuffer;
use futures::{
    sink::SinkExt,
    stream::{self, StreamExt},
    task::{noop_waker_ref, AtomicWaker},
};
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

struct Defer<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        self.0.take();
    }
}

macro_rules! defer {
    { $code:expr } => { let _defer = Defer(Some(move || { $code })); }
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
                defer!{ ltx.send(()).unwrap() };
                $tasks.await;
            });
            cnt += 1;
        )*
        for _ in 0..cnt {
            rx.recv().unwrap();
        }
    }};
}

const COUNT: usize = 1024;

#[test]
fn push_pop() {
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<usize>::new(2).split_async();
    execute_concurrently!(
        2,
        async move {
            for i in 0..COUNT {
                prod.push(i).await;
            }
        },
        async move {
            for i in 0..COUNT {
                assert_eq!(cons.pop().await, i);
            }
        },
    );
}

#[test]
fn push_pop_slice() {
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<usize>::new(3).split_async();
    execute_concurrently!(
        2,
        async move {
            let data = (0..COUNT).collect::<Vec<_>>();
            prod.push_slice(&data).await;
        },
        async move {
            let mut data = vec![0; COUNT];
            cons.pop_slice(&mut data).await;
            assert!(data.into_iter().eq(0..COUNT));
        },
    );
}

#[test]
fn sink_stream() {
    let (mut prod, mut cons) = AsyncHeapRingBuffer::<usize>::new(3).split_async();
    execute_concurrently!(
        2,
        async move {
            let mut src = stream::iter(0..COUNT).map(Ok);
            prod.sink().send_all(&mut src).await.unwrap();
        },
        async move {
            assert_eq!(
                cons.stream()
                    .take(COUNT)
                    .fold(0, |s, x| async move {
                        std::println!("{}", x);
                        assert_eq!(s, x);
                        s + 1
                    })
                    .await,
                COUNT
            );
        },
    );
}
/*
#[test]
fn transfer() {
    let (mut src_prod, mut src_cons) = AsyncHeapRingBuffer::<usize>::new(3).split_async();
    let (mut dst_prod, mut dst_cons) = AsyncHeapRingBuffer::<usize>::new(5).split_async();
    let (done_send, done_recv) = AsyncHeapRingBuffer::<usize>::new(1).split_async();
    execute_concurrently!(
        3,
        async move {
            src_prod.sink().send_all(0..COUNT).await;
        },
        async move {
            let mut data = vec![0; COUNT];
            cons.pop_slice(&mut data).await;
            assert!(data.into_iter().eq(0..COUNT));
        },
    );
}
*/
