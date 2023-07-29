use crate::{async_transfer, traits::*, AsyncHeapRb};
use core::sync::atomic::{AtomicUsize, Ordering};
use futures::task::{noop_waker_ref, AtomicWaker};
use std::{vec, vec::Vec};

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

const COUNT: usize = 16;

#[test]
fn push_pop() {
    use std::println;
    let (prod, cons) = AsyncHeapRb::<usize>::new(2).split();
    execute!(
        async move {
            let mut prod = prod;
            for i in 0..COUNT {
                prod.push(i).await.unwrap();
            }
            println!("push");
        },
        async move {
            let mut cons = cons;
            for i in 0..COUNT {
                assert_eq!(cons.pop().await.unwrap(), i);
            }
            println!("pop: {}", cons.is_closed());
            assert!(cons.pop().await.is_none());
        },
    );
}

#[test]
fn push_pop_slice() {
    let (prod, cons) = AsyncHeapRb::<usize>::new(3).split();
    execute!(
        async move {
            let mut prod = prod;
            let data = (0..COUNT).collect::<Vec<_>>();
            prod.push_slice_all(&data).await.unwrap();
        },
        async move {
            let mut cons = cons;
            let mut data = vec![0; COUNT + 1];
            let count = cons.pop_slice_all(&mut data).await.unwrap_err();
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
    execute!(
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
    execute!(
        async move {
            let mut prod = prod;
            let data = input.collect::<Vec<_>>();
            prod.write_all(&data).await.unwrap();
        },
        async move {
            let mut cons = cons;
            let mut data = Vec::new();
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
    execute!(
        async move {
            let mut prod = src_prod;
            assert!(prod.push_iter_all(0..COUNT).await);
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

#[test]
fn wait() {
    let (mut prod, mut cons) = AsyncHeapRb::<usize>::new(3).split();
    let stage = AtomicUsize::new(0);
    execute!(
        async {
            prod.push(0).await.unwrap();
            assert_eq!(stage.fetch_add(1, Ordering::SeqCst), 0);
            prod.push(1).await.unwrap();

            prod.wait_vacant(2).await;
            assert_eq!(stage.fetch_add(1, Ordering::SeqCst), 2);
        },
        async {
            cons.wait_occupied(2).await;
            assert_eq!(stage.fetch_add(1, Ordering::SeqCst), 1);

            cons.pop().await.unwrap();
        },
    );
}

#[test]
fn close_prod() {
    let (mut prod, mut cons) = AsyncHeapRb::<usize>::new(1).split();
    prod.try_push(123).unwrap();
    execute!(
        async move {
            assert_eq!(cons.pop().await, Some(123));
            assert_eq!(cons.pop().await, None);
            assert!(cons.is_closed());
        },
        async move {
            prod.wait_vacant(1).await;
            prod.close();
        },
    );
}

#[test]
fn close_cons() {
    let (mut prod, mut cons) = AsyncHeapRb::<usize>::new(1).split();
    execute!(
        async move {
            prod.push(123).await.unwrap();
            assert!(prod.push(321).await.is_err());
            assert!(prod.is_closed());
        },
        async move {
            cons.wait_occupied(1).await;
            assert_eq!(cons.as_slices().0[0], 123);
            cons.close();
        },
    );
}
