use crate::{AsyncHeapRb, async_transfer, traits::*};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures::{future::FusedFuture, task::noop_waker_ref};

fn poll<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(noop_waker_ref()))
}

#[test]
fn pop_delivers_buffered_data_and_eof_before_terminating() {
    let (mut prod, mut cons) = AsyncHeapRb::<u8>::new(1).split();
    let mut pop = cons.pop();
    assert_eq!(poll(&mut pop), Poll::Pending);
    prod.try_push(7).unwrap();
    drop(prod);
    assert!(!pop.is_terminated());
    assert_eq!(poll(&mut pop), Poll::Ready(Some(7)));
    assert!(pop.is_terminated());
    drop(pop);

    let mut eof = cons.pop();
    assert!(!eof.is_terminated());
    assert_eq!(poll(&mut eof), Poll::Ready(None));
    assert!(eof.is_terminated());
}

#[test]
fn wait_vacant_terminates_on_space_or_close() {
    for close in [false, true] {
        let (mut prod, mut cons) = AsyncHeapRb::<u8>::new(1).split();
        prod.try_push(7).unwrap();
        let mut wait = prod.wait_vacant(1);
        assert!(!wait.is_terminated());
        assert_eq!(poll(&mut wait), Poll::Pending);
        if close {
            drop(cons);
        } else {
            assert_eq!(cons.try_pop(), Some(7));
        }
        assert_eq!(poll(&mut wait), Poll::Ready(()));
        assert!(wait.is_terminated());
    }
}

#[test]
fn push_iter_delivers_closed_result_before_terminating() {
    let (mut prod, cons) = AsyncHeapRb::<u8>::new(1).split();
    let mut push = prod.push_iter_all(0..3);
    assert_eq!(poll(&mut push), Poll::Pending);
    drop(cons);
    assert!(!push.is_terminated());
    assert_eq!(poll(&mut push), Poll::Ready(false));
    assert!(push.is_terminated());
}

#[test]
fn select_drains_closed_producer() {
    let (mut prod, mut cons) = AsyncHeapRb::<u8>::new(1).split();
    prod.try_push(7).unwrap();
    drop(prod);
    futures::executor::block_on(async {
        let mut pop = cons.pop();
        futures::select! {
            item = pop => assert_eq!(item, Some(7)),
            complete => panic!("buffered item skipped"),
        }
    });
}

#[cfg(feature = "std")]
#[test]
fn empty_io_completes_without_changing_buffer() {
    use futures::io::{AsyncRead, AsyncWrite};
    for full in [false, true] {
        for closed in [false, true] {
            let (mut prod, mut cons) = AsyncHeapRb::<u8>::new(1).split();
            if full {
                prod.try_push(7).unwrap();
            }
            if closed {
                prod.close();
            }
            let mut cx = Context::from_waker(noop_waker_ref());
            assert!(matches!(
                AsyncRead::poll_read(Pin::new(&mut cons), &mut cx, &mut []),
                Poll::Ready(Ok(0))
            ));
            assert!(matches!(
                AsyncConsumer::poll_read(Pin::new(&mut cons), &mut cx, &mut []),
                Poll::Ready(Ok(0))
            ));
            assert_eq!(cons.try_pop(), if full { Some(7) } else { None });

            let (mut prod, mut cons) = AsyncHeapRb::<u8>::new(1).split();
            if full {
                prod.try_push(7).unwrap();
            }
            if closed {
                cons.close();
            }
            assert!(matches!(
                AsyncWrite::poll_write(Pin::new(&mut prod), &mut cx, &[]),
                Poll::Ready(Ok(0))
            ));
            assert!(matches!(
                AsyncProducer::poll_write(Pin::new(&mut prod), &mut cx, &[]),
                Poll::Ready(Ok(0))
            ));
            assert_eq!(prod.occupied_len(), usize::from(full));
        }
    }
}

#[test]
fn transfer_counts_only_delivered_items() {
    for len in [0, 2] {
        for count in [None, Some(0), Some(1), Some(5)] {
            let (mut src_prod, mut src) = AsyncHeapRb::<u8>::new(4).split();
            src_prod.push_iter(0..len);
            drop(src_prod);
            let (mut dst, mut dst_cons) = AsyncHeapRb::<u8>::new(4).split();
            let expected = usize::from(len).min(count.unwrap_or(usize::MAX));
            assert_eq!(futures::executor::block_on(async_transfer(&mut src, &mut dst, count)), expected);
            assert!(dst_cons.pop_iter().eq(0..expected as u8));
        }
    }
    let (mut src_prod, mut src) = AsyncHeapRb::<u8>::new(1).split();
    src_prod.try_push(7).unwrap();
    let (mut dst, dst_cons) = AsyncHeapRb::<u8>::new(1).split();
    drop(dst_cons);
    assert_eq!(futures::executor::block_on(async_transfer(&mut src, &mut dst, None)), 0);
}
