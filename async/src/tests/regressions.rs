use crate::{AsyncHeapRb, async_transfer, traits::*};
use alloc::string::String;
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
        assert_eq!(
            poll(&mut wait),
            Poll::Ready(if close { Err(ringbuf::error::WaitError::Closed) } else { Ok(()) })
        );
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
    assert_eq!(
        poll(&mut push),
        Poll::Ready(Err(ringbuf::error::TransferError {
            completed: 1,
            reason: ringbuf::error::WaitError::Closed
        }))
    );
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
            let result = futures::executor::block_on(async_transfer(&mut src, &mut dst, count));
            assert_eq!(result.unwrap_or_else(|e| e.completed), expected);
            assert!(dst_cons.pop_iter().eq(0..expected as u8));
        }
    }
    let (mut src_prod, mut src) = AsyncHeapRb::<u8>::new(1).split();
    src_prod.try_push(7).unwrap();
    let (mut dst, dst_cons) = AsyncHeapRb::<u8>::new(1).split();
    drop(dst_cons);
    assert_eq!(
        futures::executor::block_on(async_transfer(&mut src, &mut dst, None))
            .unwrap_err()
            .completed,
        0
    );
    assert_eq!(src.try_pop(), Some(7));
}

#[test]
fn invalid_thresholds_and_empty_transfers_complete_immediately() {
    use ringbuf::error::WaitError;
    let (mut p, mut c) = AsyncHeapRb::<u8>::new(2).split();
    assert_eq!(
        poll(&mut p.wait_vacant(3)),
        Poll::Ready(Err(WaitError::TooLarge { requested: 3, capacity: 2 }))
    );
    assert_eq!(
        poll(&mut c.wait_occupied(3)),
        Poll::Ready(Err(WaitError::TooLarge { requested: 3, capacity: 2 }))
    );
    c.close();
    assert_eq!(poll(&mut p.wait_vacant(1)), Poll::Ready(Err(WaitError::Closed)));
    assert_eq!(poll(&mut p.push_all(&[])), Poll::Ready(Ok(0)));
    assert_eq!(poll(&mut p.wait_vacant(0)), Poll::Ready(Ok(())));
}

#[test]
fn deferred_threshold_wakeups_and_recovery_use_the_marker_backend() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::task::{Wake, Waker};
    struct Count(AtomicUsize);
    impl Wake for Count {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let counter = Arc::new(Count(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let (mut p, mut c) = AsyncHeapRb::<u8>::new(3).split();
    let mut wait = c.wait_occupied(2);
    assert_eq!(Pin::new(&mut wait).poll(&mut cx), Poll::Pending);
    let mut d = p.defer();
    d.try_push(1).unwrap();
    assert_eq!(counter.0.load(Ordering::Relaxed), 0);
    d.commit();
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    assert_eq!(Pin::new(&mut wait).poll(&mut cx), Poll::Pending);
    d.try_push(2).unwrap();
    drop(d);
    assert_eq!(counter.0.load(Ordering::Relaxed), 2);
    assert_eq!(Pin::new(&mut wait).poll(&mut cx), Poll::Ready(Ok(())));
    drop(wait);
    let d = c.defer();
    core::mem::forget(d);
    let mut wait = c.wait_occupied(1); // recover the abandoned claim before waiting
    assert_eq!(poll(&mut wait), Poll::Pending);
    assert_eq!(p.vacant_len(), 3);
    p.try_push(3).unwrap();
    assert_eq!(poll(&mut wait), Poll::Ready(Ok(())));
}

#[test]
fn closed_endpoint_stays_inert_after_reacquisition() {
    use ringbuf::endpoint::Endpoint;
    let (mut old, mut c) = AsyncHeapRb::<u8>::new(2).split();
    let handle = old.rb_handle().clone();
    old.close();
    old.close();
    assert_eq!(poll(&mut old.push(7)), Poll::Ready(Err(7)));
    let mut new = crate::AsyncProd::try_new(handle).ok().unwrap();
    assert_eq!(old.try_push(8), Err(8));
    assert_eq!(poll(&mut old.push(8)), Poll::Ready(Err(8)));
    new.try_push(9).unwrap();
    assert_eq!(c.try_pop(), Some(9));
}

#[test]
fn panicking_waker_runs_after_publication() {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::Arc,
        task::{Wake, Waker},
    };
    struct Panic;
    impl Wake for Panic {
        fn wake(self: Arc<Self>) {
            panic!("waker");
        }
    }
    let (mut p, mut c) = AsyncHeapRb::<String>::new(2).split();
    let mut wait = c.wait_occupied(1);
    let waker = Waker::from(Arc::new(Panic));
    assert_eq!(Pin::new(&mut wait).poll(&mut Context::from_waker(&waker)), Poll::Pending);
    let mut d = p.defer();
    d.try_push(String::from("published")).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| d.commit())).is_err());
    assert_eq!(d.pending_len(), 0);
    assert_eq!(poll(&mut wait), Poll::Ready(Ok(())));
    drop(wait);
    assert_eq!(c.try_pop().as_deref(), Some("published"));
}

#[test]
fn async_markers_accept_local_indices() {
    let mut rb = ringbuf::Rb::<ringbuf::storage::Array<i32, 2>, ringbuf::indices::LocalIndices, crate::AsyncMarkers>::default();
    let (mut p, mut c) = rb.split_ref();
    assert_eq!(poll(&mut p.push(42)), Poll::Ready(Ok(())));
    assert_eq!(poll(&mut c.pop()), Poll::Ready(Some(42)));
}

#[test]
fn cancellation_does_not_remove_an_item_waiting_for_destination_space() {
    let (mut p, mut src) = AsyncHeapRb::<u8>::new(2).split();
    let (mut dst, mut c) = AsyncHeapRb::<u8>::new(1).split();
    p.try_push(7).unwrap();
    dst.try_push(9).unwrap();
    let mut transfer = async_transfer(&mut src, &mut dst, Some(1));
    assert_eq!(poll(&mut transfer), Poll::Pending);
    assert_eq!(transfer.count(), 0);
    drop(transfer);
    assert_eq!(src.try_pop(), Some(7));
    assert_eq!(c.try_pop(), Some(9));
}

#[test]
fn bounded_collection_moves_noncopy_items_and_reports_progress() {
    let (mut p, mut c) = AsyncHeapRb::<String>::new(3).split();
    p.push_iter([String::from("a"), String::from("b"), String::from("c")].into_iter());
    let mut output = alloc::vec::Vec::new();
    assert!(matches!(poll(&mut c.pop_into_vec(&mut output, 2)), Poll::Ready(Ok(2))));
    assert_eq!(output, ["a", "b"]);
    assert_eq!(c.try_pop().as_deref(), Some("c"));
    let mut future = c.pop_into_vec(&mut output, 3);
    assert!(poll(&mut future).is_pending());
    assert_eq!(future.count(), 0);
}
