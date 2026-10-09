use super::Rb;
use crate::{storage::Array, traits::*};

#[test]
fn iter() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (mut prod, mut cons) = rb.split_ref();

    prod.try_push(10).unwrap();
    prod.try_push(20).unwrap();

    let sum: i32 = cons.iter().sum();

    let first = cons.try_pop().expect("First item is not available");
    let second = cons.try_pop().expect("Second item is not available");

    assert_eq!(sum, first + second);
}

#[test]
fn iter_mut() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (mut prod, mut cons) = rb.split_ref();

    prod.try_push(10).unwrap();
    prod.try_push(20).unwrap();

    for v in cons.iter_mut() {
        *v *= 2;
    }

    let sum: i32 = cons.iter().sum();

    let first = cons.try_pop().expect("First item is not available");
    let second = cons.try_pop().expect("Second item is not available");

    assert_eq!(sum, first + second);
}

#[test]
fn pop_iter() {
    let mut rb = Rb::<Array<i32, 3>>::default();
    let (mut prod, mut cons) = rb.split_ref();

    prod.try_push(0).unwrap();
    prod.try_push(1).unwrap();
    for (i, v) in cons.pop_iter().enumerate() {
        assert_eq!(i as i32, v);
    }

    prod.try_push(2).unwrap();
    prod.try_push(3).unwrap();
    for (i, v) in cons.pop_iter().enumerate() {
        assert_eq!(i as i32 + 2, v);
    }
    assert!(prod.is_empty());
}

#[test]
fn push_pop_iter_partial() {
    let mut rb = Rb::<Array<i32, 4>>::default();
    let (mut prod, mut cons) = rb.split_ref();

    prod.try_push(0).unwrap();
    prod.try_push(1).unwrap();
    prod.try_push(2).unwrap();
    for (i, v) in (0..2).zip(cons.pop_iter()) {
        assert_eq!(i, v);
    }

    prod.try_push(3).unwrap();
    prod.try_push(4).unwrap();
    prod.try_push(5).unwrap();
    for (i, v) in (2..5).zip(cons.pop_iter()) {
        assert_eq!(i, v);
    }
    assert_eq!(cons.try_pop().unwrap(), 5);
    assert!(prod.is_empty());
}

#[test]
#[allow(deprecated)]
fn pop_iter_publishes_each_item_and_keeps_its_snapshot() {
    let mut rb = Rb::<Array<i32, 3>>::default();
    let (mut prod, mut cons) = rb.split_ref();
    prod.push_slice(&[0, 1, 2]);
    {
        let mut iter = cons.pop_iter();
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.next(), Some(0));
        // Reuse the freed slot while the iterator is still alive.
        prod.try_push(3).unwrap();
        iter.commit();
        iter.commit();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.len(), 0);
        prod.try_push(4).unwrap();
        assert_eq!(iter.next(), None);
    }
    assert_eq!(cons.try_pop(), Some(3));
    assert_eq!(cons.try_pop(), Some(4));
}

#[test]
#[allow(deprecated)]
fn forgotten_pop_iter_drops_each_item_once() {
    use core::{cell::Cell, mem};

    struct Item<'a>(&'a Cell<usize>);
    impl Drop for Item<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    for mode in 0..3 {
        for offset in [0, 3] {
            for count in 0..=3 {
                let drops: [Cell<usize>; 3] = core::array::from_fn(|_| Cell::new(0));
                let mut rb = Rb::<Array<Item<'_>, 4>>::default();
                for _ in 0..offset {
                    rb.try_push(Item(&drops[0])).ok().unwrap();
                    drop(rb.try_pop().unwrap());
                }
                drops[0].set(0);
                for counter in &drops {
                    rb.try_push(Item(counter)).ok().unwrap();
                }
                // Check the owner, a cached endpoint, and the deprecated wrapper.
                macro_rules! consume {
                    ($cons:expr) => {{
                        let mut iter = $cons.pop_iter();
                        for _ in 0..count {
                            drop(iter.next().unwrap());
                        }
                        #[allow(
                            clippy::forget_non_drop,
                            reason = "Regression test: soundness must not depend on the iterator's destructor"
                        )]
                        mem::forget(iter);
                    }};
                }
                match mode {
                    0 => consume!(rb),
                    1 => {
                        let (_prod, mut cons) = rb.split_ref();
                        consume!(cons);
                    }
                    _ => {
                        let (_prod, cons) = rb.split_ref();
                        let mut cons = cons.into_deferred();
                        consume!(cons);
                        drop(cons);
                    }
                }
                assert_eq!(rb.occupied_len(), 3 - count);
                drop(rb);
                for counter in &drops {
                    assert_eq!(counter.get(), 1);
                }
            }
        }
    }
}

#[cfg(feature = "std")]
#[test]
fn pop_iter_allows_concurrent_slot_reuse() {
    use crate::SharedRb;
    use alloc::boxed::Box;
    use std::{sync::mpsc, thread};

    let mut rb = SharedRb::<Array<Box<usize>, 3>>::default();
    rb.push_iter((0..3).map(Box::new));
    let (mut prod, mut cons) = rb.split_ref();
    let (released_tx, released_rx) = mpsc::channel();
    let (reused_tx, reused_rx) = mpsc::channel();
    thread::scope(|scope| {
        scope.spawn(move || {
            for value in 3..6 {
                released_rx.recv().unwrap();
                prod.try_push(Box::new(value)).unwrap();
                reused_tx.send(()).unwrap();
            }
        });
        {
            let mut iter = cons.pop_iter();
            for value in 0..3 {
                assert_eq!(*iter.next().unwrap(), value);
                released_tx.send(()).unwrap();
                reused_rx.recv().unwrap();
            }
            assert_eq!(iter.next(), None);
        }
        assert!(cons.pop_iter().map(|value| *value).eq(3..6));
    });
}
