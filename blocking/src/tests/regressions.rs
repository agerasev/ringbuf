use crate::{BlockingHeapRb, WaitError, sync::NO_WAIT, traits::*};
use std::io::{Read, Write};

#[test]
fn read_keeps_final_batch_when_producer_closes_after_snapshot() {
    use crate::{BlockingRb, sync::StdSemaphore};
    use core::{
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
    };
    use ringbuf::{
        SharedRb,
        storage::{Heap, Storage},
    };
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
    let rb = BlockingRb::<_, StdSemaphore>::from(unsafe { SharedRb::from_raw_parts(storage, 0, 0) });
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
