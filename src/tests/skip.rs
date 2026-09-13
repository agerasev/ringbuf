use super::Rb;
use crate::{storage::Array, traits::*};
use alloc::rc::Rc;

#[test]
fn skip() {
    // Initialize ringbuffer, prod and cons
    let mut rb = Rb::<Array<i8, 10>>::default();
    let (mut prod, mut cons) = rb.split_ref();
    let mut i = 0;

    // Fill the buffer
    for _ in 0..10 {
        prod.try_push(i).unwrap();
        i += 1;
    }

    // Pop in the middle of the buffer
    assert_eq!(cons.skip(5), 5);

    // Make sure changes are taken into account
    assert_eq!(cons.try_pop().unwrap(), 5);

    // Fill the buffer again
    for _ in 0..5 {
        prod.try_push(i).unwrap();
        i += 1;
    }

    assert_eq!(cons.skip(6), 6);
    assert_eq!(cons.try_pop().unwrap(), 12);

    // Fill the buffer again
    for _ in 0..7 {
        prod.try_push(i).unwrap();
        i += 1;
    }

    // Ask too much, delete the max number of items
    assert_eq!(cons.skip(10), 9);

    // Try to remove more than possible
    assert_eq!(cons.skip(1), 0);

    // Make sure it is still usable
    assert_eq!(cons.try_pop(), None);
    assert_eq!(prod.try_push(0), Ok(()));
    assert_eq!(cons.try_pop(), Some(0));
}

#[test]
fn skip_drop() {
    let rc = Rc::<()>::new(());

    const CAP: usize = 10;
    let mut rb = Rb::<Array<Rc<()>, CAP>>::default();
    let (mut prod, mut cons) = rb.split_ref();

    for _ in 0..CAP {
        prod.try_push(rc.clone()).unwrap();
    }

    assert_eq!(cons.occupied_len(), CAP);
    assert_eq!(Rc::strong_count(&rc), CAP + 1);

    assert_eq!(cons.skip(CAP), CAP);

    // Check ring buffer is empty
    assert_eq!(cons.occupied_len(), 0);
    // Check that items are dropped
    assert_eq!(Rc::strong_count(&rc), 1);
}

/// A panicking destructor must be removed exactly once, while later items
/// remain initialized and the buffer can still be used after unwinding.
#[cfg(feature = "std")]
#[test]
fn skip_panicking_drop() {
    use core::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    const CAP: usize = 4;
    std::thread_local! {
        static DROPS: Cell<[usize; CAP + 1]> = const { Cell::new([0; CAP + 1]) };
        static ARMED: Cell<Option<usize>> = const { Cell::new(None) };
    }

    struct Boom(usize);
    impl Drop for Boom {
        fn drop(&mut self) {
            DROPS.with(|d| {
                let mut drops = d.get();
                drops[self.0] += 1;
                d.set(drops);
            });
            if ARMED.with(|a| {
                if a.get() == Some(self.0) {
                    a.set(None);
                    true
                } else {
                    false
                }
            }) {
                panic!("item Drop panics");
            }
        }
    }

    fn check(cons: &mut impl Consumer<Item = Boom>, clear: bool, panic_at: usize) {
        ARMED.with(|a| a.set(Some(panic_at)));
        let result = catch_unwind(AssertUnwindSafe(|| if clear { cons.clear() } else { cons.skip(CAP - 1) }));
        ARMED.with(|a| a.set(None));
        assert!(result.is_err());
        assert_eq!(cons.occupied_len(), CAP - panic_at - 1);
        assert!(cons.iter().map(|item| item.0).eq(panic_at + 1..CAP));
        DROPS.with(|d| {
            for (id, drops) in d.get().into_iter().enumerate() {
                assert_eq!(drops, usize::from(id <= panic_at), "item {id}");
            }
        });
    }

    for clear in [false, true] {
        for offset in [0, CAP - 1] {
            for panic_at in 0..if clear { CAP } else { CAP - 1 } {
                // Exercise the owner, a split consumer, and a frozen consumer.
                for mode in 0..3 {
                    let mut rb = Rb::<Array<Boom, CAP>>::default();
                    for _ in 0..offset {
                        rb.try_push(Boom(CAP)).ok().unwrap();
                        drop(rb.try_pop().unwrap());
                    }
                    DROPS.with(|d| d.set([0; CAP + 1]));
                    for id in 0..CAP {
                        rb.try_push(Boom(id)).ok().unwrap();
                    }
                    match mode {
                        0 => check(&mut rb, clear, panic_at),
                        1 => {
                            let (_prod, mut cons) = rb.split_ref();
                            check(&mut cons, clear, panic_at);
                        }
                        _ => {
                            let (_prod, cons) = rb.split_ref();
                            check(&mut cons.freeze(), clear, panic_at);
                        }
                    }
                    rb.try_push(Boom(CAP)).ok().unwrap();
                    drop(rb);
                    DROPS.with(|d| assert_eq!(d.get(), [1; CAP + 1]));
                }
            }
        }
    }
}

/// A slot must remain occupied while its in-place destructor is running.
#[cfg(feature = "std")]
#[test]
fn skip_does_not_release_slot_during_drop() {
    use crate::SharedRb;
    use std::{sync::mpsc, thread};

    struct WaitOnDrop {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }

    impl Drop for WaitOnDrop {
        fn drop(&mut self) {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
        }
    }

    for clear in [false, true] {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut rb = SharedRb::<Array<WaitOnDrop, 1>>::default();
        let (mut prod, mut cons) = rb.split_ref();
        prod.try_push(WaitOnDrop {
            entered: entered_tx,
            release: release_rx,
        })
        .ok()
        .unwrap();

        thread::scope(|scope| {
            let consumer = scope.spawn(move || if clear { cons.clear() } else { cons.skip(1) });
            entered_rx.recv().unwrap();
            // Observe the slot without writing to it, so this test remains safe
            // even if a broken consumer publishes it before destruction ends.
            let vacant = prod.vacant_len();
            release_tx.send(()).unwrap();
            assert_eq!(consumer.join().unwrap(), 1);
            assert_eq!(vacant, 0, "slot released while its destructor was running");
            assert_eq!(prod.vacant_len(), 1);
        });
    }
}

/// Batch removal only visits the items occupied when it starts.
#[cfg(feature = "std")]
#[test]
fn skip_preserves_items_pushed_during_drop() {
    use crate::SharedRb;
    use alloc::boxed::Box;
    use std::{sync::mpsc, thread};

    struct OnDrop(Option<Box<dyn FnOnce() + Send>>);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            if let Some(action) = self.0.take() {
                action();
            }
        }
    }

    for clear in [false, true] {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut rb = SharedRb::<Array<OnDrop, 2>>::default();
        let (mut prod, mut cons) = rb.split_ref();
        prod.try_push(OnDrop(Some(Box::new(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }))))
        .ok()
        .unwrap();

        thread::scope(|scope| {
            let consumer = scope.spawn(move || {
                let removed = if clear { cons.clear() } else { cons.skip(usize::MAX) };
                (cons, removed)
            });
            entered_rx.recv().unwrap();
            let pushed = prod.try_push(OnDrop(None)).is_ok();
            release_tx.send(()).unwrap();
            let (mut cons, removed) = consumer.join().unwrap();
            assert!(pushed);
            assert_eq!(removed, 1);
            assert!(cons.try_pop().is_some());
            assert!(cons.is_empty());
        });
    }
}
