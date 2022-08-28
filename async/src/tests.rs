use crate::{async_transfer, AsyncHeapRb};
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

struct Defer<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        self.0.take().unwrap()();
    }
}

macro_rules! defer {
    { $code:expr } => { let _defer = Defer(Some(move || { $code })); }
}

#[allow(unused_macros)]
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
            .pool_size($pool_size)
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
    let (prod, cons) = AsyncHeapRb::<usize>::new(2).split();
    execute_concurrently!(
        2,
        async move {
            let mut prod = prod;
            for i in 0..COUNT {
                prod.push(i).await.unwrap();
            }
        },
        async move {
            let mut cons = cons;
            for i in 0..COUNT {
                assert_eq!(cons.pop().await.unwrap(), i);
            }
            assert!(cons.pop().await.is_none());
        },
    );
}

#[test]
fn push_pop_slice() {
    let (prod, cons) = AsyncHeapRb::<usize>::new(3).split();
    execute_concurrently!(
        2,
        async move {
            let mut prod = prod;
            let data = (0..COUNT).collect::<Vec<_>>();
            prod.push_slice(&data).await.unwrap();
        },
        async move {
            let mut cons = cons;
            let mut data = vec![0; COUNT + 1];
            let count = cons.pop_slice(&mut data).await.unwrap_err();
            assert_eq!(count, COUNT);
            assert!(data.into_iter().take(COUNT).eq(0..COUNT));
        },
    );
}

#[test]
fn sink_stream() {
    use futures::{
        sink::SinkExt,
        stream::{self, StreamExt},
    };
    let (prod, cons) = AsyncHeapRb::<usize>::new(2).split();
    execute_concurrently!(
        2,
        async move {
            let mut prod = prod;
            let mut input = stream::iter(0..COUNT).map(Ok);
            prod.send_all(&mut input).await.unwrap();
        },
        async move {
            let cons = cons;
            assert_eq!(
                cons.fold(0, |s, x| async move {
                    assert_eq!(s, x);
                    s + 1
                })
                .await,
                COUNT
            );
        },
    );
}

#[test]
fn read_write() {
    use futures::{AsyncReadExt, AsyncWriteExt};
    let (prod, cons) = AsyncHeapRb::<u8>::new(3).split();
    let input = (0..255).cycle().take(COUNT);
    let output = input.clone();
    execute_concurrently!(
        2,
        async move {
            let mut prod = prod;
            let data = input.collect::<Vec<_>>();
            prod.write_all(&data).await.unwrap();
        },
        async move {
            let mut cons = cons;
            let mut data = vec![0; COUNT + 1];
            let count = cons.read_to_end(&mut data).await.unwrap();
            assert_eq!(count, COUNT);
            assert!(data.into_iter().take(COUNT).eq(output));
        },
    );
}

#[test]
fn transfer() {
    use futures::stream::StreamExt;
    let (src_prod, src_cons) = AsyncHeapRb::<usize>::new(3).split();
    let (dst_prod, dst_cons) = AsyncHeapRb::<usize>::new(5).split();
    execute_concurrently!(
        3,
        async move {
            let mut prod = src_prod;
            prod.push_iter(0..COUNT).await.unwrap();
        },
        async move {
            let mut src = src_cons;
            let mut dst = dst_prod;
            async_transfer(&mut src, &mut dst, None).await
        },
        async move {
            let cons = dst_cons;
            assert_eq!(
                cons.fold(0, |s, x| async move {
                    assert_eq!(s, x);
                    s + 1
                })
                .await,
                COUNT
            );
        },
    );
}
