use crate::{BlockingHeapRb, WaitError, sync::NO_WAIT, traits::*};
use std::io::{Read, Write};

#[test]
fn read_keeps_final_batch_when_producer_closes_after_snapshot() {
    use crate::{BlockingRb, sync::StdSemaphore};
    use core::{
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
    };
    use ringbuf::storage::{Heap, Storage};
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    struct Gate {
        armed: AtomicBool,
        snapshot: Barrier,
        closed: Barrier,
    }
    struct PausingStorage {
        inner: Heap<u8>,
        gate: Arc<Gate>,
    }
    // SAFETY: All memory access is delegated to Heap; the length and pointer
    // remain constant. The barriers only control when len() returns.
    unsafe impl Storage for PausingStorage {
        type Item = u8;
        fn len(&self) -> usize {
            let len = self.inner.len();
            if self.gate.armed.swap(false, Ordering::SeqCst) {
                // occupied_slices() queries capacity after loading the write
                // index. Pause that empty snapshot until the final write and
                // producer close have both completed.
                self.gate.snapshot.wait();
                self.gate.closed.wait();
            }
            len
        }
        fn as_mut_ptr(&self) -> *mut MaybeUninit<u8> {
            self.inner.as_mut_ptr()
        }
    }

    let gate = Arc::new(Gate {
        armed: AtomicBool::new(false),
        snapshot: Barrier::new(2),
        closed: Barrier::new(2),
    });
    let storage = PausingStorage {
        inner: Heap::new(3),
        gate: gate.clone(),
    };
    // SAFETY: The storage is nonempty and entirely uninitialized.
    let rb = unsafe { BlockingRb::<_, StdSemaphore>::from_raw_parts(storage, 0, 0) };
    let (mut prod, mut cons) = rb.split();
    let producer = thread::spawn({
        let gate = gate.clone();
        move || {
            gate.snapshot.wait();
            assert_eq!(prod.push_slice(b"end"), 3);
            drop(prod);
            gate.closed.wait();
        }
    });
    gate.armed.store(true, Ordering::SeqCst);
    let mut bytes = [0; 3];
    let count = cons.read(&mut bytes).unwrap();
    producer.join().unwrap();
    assert_eq!(count, 3);
    assert_eq!(&bytes, b"end");
    assert_eq!(cons.read(&mut bytes).unwrap(), 0);
}

#[test]
fn closed_producer_is_drained_before_reporting_closed() {
    let (mut prod, mut cons) = BlockingHeapRb::<u8>::new(3).split();
    prod.push_slice(b"end");
    drop(prod);
    cons.set_timeout(NO_WAIT);
    assert_eq!(cons.wait_occupied(3), Ok(()));
    for byte in b"end" {
        assert_eq!(cons.pop(), Ok(*byte));
    }
    assert_eq!(cons.pop(), Err(WaitError::Closed));
    assert_eq!(cons.wait_occupied(1), Err(WaitError::Closed));
}

#[test]
fn empty_io_completes_without_waiting() {
    for full in [false, true] {
        let (mut prod, mut cons) = BlockingHeapRb::<u8>::new(1).split();
        prod.set_timeout(NO_WAIT);
        cons.set_timeout(NO_WAIT);
        if full {
            prod.try_push(7).unwrap();
        }
        assert_eq!(cons.read(&mut []).unwrap(), 0);
        assert_eq!(prod.write(&[]).unwrap(), 0);
        assert_eq!(cons.try_pop(), if full { Some(7) } else { None });
    }
}

#[test]
fn thresholds_closed_writes_and_streaming_progress() {
    use crate::BlockingHeapRb;
    use ringbuf::error::{TransferError, WaitError};
    let (mut p, mut c) = BlockingHeapRb::<u8>::new(2).split();
    p.set_timeout(Some(core::time::Duration::ZERO));
    assert_eq!(p.wait_vacant(3), Err(WaitError::TooLarge { requested: 3, capacity: 2 }));
    assert_eq!(c.wait_occupied(3), Err(WaitError::TooLarge { requested: 3, capacity: 2 }));
    assert_eq!(
        p.push_all(&[1, 2, 3]),
        Err(TransferError {
            completed: 2,
            reason: WaitError::TimedOut
        })
    );
    drop(p);
    let mut dst = [0; 3];
    assert_eq!(
        c.pop_all(&mut dst),
        Err(TransferError {
            completed: 2,
            reason: WaitError::Closed
        })
    );
    assert_eq!(dst, [1, 2, 0]);
    let (mut p, c) = BlockingHeapRb::<u8>::new(1).split();
    drop(c);
    assert_eq!(p.push(9), Err((WaitError::Closed, 9)));
    assert_eq!(p.wait_vacant(1), Err(WaitError::Closed));
    assert_eq!(p.push_all(&[]), Ok(0));
}

#[test]
fn expired_deadline_checks_readiness_once_without_blocking() {
    let (mut p, mut c) = crate::BlockingHeapRb::<u8>::new(1).split();
    let past = std::time::Instant::now();
    c.set_deadline(Some(past));
    assert_eq!(c.pop(), Err(WaitError::TimedOut));
    p.push(5).unwrap();
    assert_eq!(c.pop(), Ok(5));
}

#[test]
fn bounded_collection_and_timed_out_iterator_preserve_values() {
    let (mut p, mut c) = crate::BlockingHeapRb::<i32>::new(2).split();
    p.set_timeout(Some(core::time::Duration::ZERO));
    let mut values = (0..4).peekable();
    let error = p.push_all_iter(&mut values).unwrap_err();
    assert_eq!((error.completed, error.reason), (2, WaitError::TimedOut));
    assert_eq!(values.next(), Some(2));
    let mut output = alloc::vec::Vec::new();
    assert_eq!(c.pop_into_vec(&mut output, 1).unwrap(), 1);
    assert_eq!(output, [0]);
    assert_eq!(c.try_pop(), Some(1));
}
