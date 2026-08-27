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

/// A panicking `Drop` must not leave already-destroyed items inside the
/// occupied range. `skip` and `clear` advanced the read index only after the
/// loop, so an unwind left the ring buffer's own `Drop` to destroy them again.
#[cfg(feature = "std")]
#[test]
fn skip_panicking_drop() {
    use core::cell::Cell;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    std::thread_local! {
        static DROPS: Cell<usize> = const { Cell::new(0) };
        static ARMED: Cell<bool> = const { Cell::new(false) };
    }

    struct Boom;

    impl Drop for Boom {
        fn drop(&mut self) {
            DROPS.with(|d| d.set(d.get() + 1));
            if ARMED.with(|a| a.replace(false)) {
                panic!("item Drop panics");
            }
        }
    }

    const CAP: usize = 4;

    for (name, count) in [("clear", CAP), ("skip", 2)] {
        DROPS.with(|d| d.set(0));

        let mut rb = Rb::<Array<Boom, CAP>>::default();
        for _ in 0..CAP {
            rb.try_push(Boom).ok().unwrap();
        }

        ARMED.with(|a| a.set(true));
        let r = catch_unwind(AssertUnwindSafe(|| {
            rb.skip(count);
        }));
        ARMED.with(|a| a.set(false));
        assert!(r.is_err(), "{}: the armed Drop should have panicked", name);

        drop(rb);

        // Four items exist. Fewer drops mean a leak, which is sound; more mean
        // an item was destroyed twice.
        let drops = DROPS.with(|d| d.get());
        assert!(
            drops <= CAP,
            "{}: {} drops for {} items - an item was destroyed twice",
            name,
            drops,
            CAP
        );
    }
}
