#![allow(deprecated)]

use super::Rb;
use crate::{
    CachingProd, Cons, Prod,
    storage::Array,
    traits::*,
    wrap::{FrozenCons, FrozenProd, Wrap},
};
use core::{cell::Cell, mem};

#[test]
fn producer_publishes_without_commit() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (prod, mut cons) = rb.split_ref();
    let mut prod = prod.freeze();
    prod.try_push(0).unwrap();
    prod.try_push(1).unwrap();
    assert_eq!(cons.try_pop(), Some(0));
    // The cached producer must fetch progress when its old snapshot is full.
    prod.try_push(2).unwrap();
    prod.commit();
    prod.fetch();
    prod.sync();
    assert_eq!(cons.try_pop(), Some(1));
    assert_eq!(cons.try_pop(), Some(2));
    assert_eq!(cons.try_pop(), None);
}

#[test]
fn discard_does_not_retract_published_items() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (prod, mut cons) = rb.split_ref();
    let mut prod = prod.freeze();
    prod.try_push(0).unwrap();
    prod.discard();
    assert_eq!(cons.try_pop(), Some(0));
    prod.try_push(1).unwrap();
    assert_eq!(cons.try_pop(), Some(1));
    // Discarding after the consumer has advanced must not rewind the write index.
    prod.discard();
    assert_eq!(cons.try_pop(), None);
    prod.try_push(2).unwrap();
    assert_eq!(cons.try_pop(), Some(2));
}

#[test]
fn consumer_publishes_without_commit() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (mut prod, cons) = rb.split_ref();
    let mut cons = cons.freeze();
    prod.push_slice(&[0, 1]);
    // The consumer starts with an empty cached snapshot and must fetch automatically.
    assert_eq!(cons.try_pop(), Some(0));
    prod.try_push(2).unwrap();
    cons.commit();
    cons.fetch();
    cons.sync();
    assert_eq!(cons.try_pop(), Some(1));
    assert_eq!(cons.try_pop(), Some(2));
    assert_eq!(cons.try_pop(), None);
}

struct CountDrop<'a>(&'a Cell<usize>);
impl Drop for CountDrop<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn forgotten_consumer_does_not_drop_items_twice() {
    for mode in 0..3 {
        for offset in [0, 3] {
            let drops: [Cell<usize>; 3] = core::array::from_fn(|_| Cell::new(0));
            let mut rb = Rb::<Array<CountDrop<'_>, 4>>::default();
            for _ in 0..offset {
                rb.try_push(CountDrop(&drops[0])).ok().unwrap();
                drop(rb.try_pop().unwrap());
            }
            drops[0].set(0);
            let mut prod = CachingProd::new(&rb);
            for counter in &drops {
                prod.try_push(CountDrop(counter)).ok().unwrap();
            }
            let mut cons = match mode {
                0 => crate::CachingCons::new(&rb).freeze(),
                1 => Cons::new(&rb).freeze(),
                _ => FrozenCons::new(&rb),
            };
            let item = cons.try_pop().unwrap();
            mem::forget(cons);
            drop(item);
            drop(prod);
            assert_eq!(rb.occupied_len(), 2);
            assert_eq!(drops[0].get(), 1);
            drop(rb);
            for counter in &drops {
                assert_eq!(counter.get(), 1);
            }
        }
    }
}

#[test]
fn forgotten_producer_keeps_items_owned_by_buffer() {
    let drops = Cell::new(0);
    let rb = Rb::<Array<CountDrop<'_>, 2>>::default();
    let mut prod = FrozenProd::new(&rb);
    prod.try_push(CountDrop(&drops)).ok().unwrap();
    mem::forget(prod);
    assert_eq!(rb.occupied_len(), 1);
    drop(rb);
    assert_eq!(drops.get(), 1);
}

#[test]
fn conversion_and_extraction_preserve_endpoint_rights() {
    let rb = Rb::<Array<i32, 2>>::default();
    let prod = Prod::new(&rb).freeze();
    let cons = Cons::new(&rb).freeze();
    let obs = prod.observe();
    assert!(obs.write_is_held() && obs.read_is_held());
    let rb_ref = prod.into_rb_ref();
    assert!(!obs.write_is_held() && obs.read_is_held());
    let mut prod = FrozenProd::new(rb_ref);
    prod.try_push(1).unwrap();
    drop(prod);
    drop(cons);
    assert!(!obs.write_is_held() && !obs.read_is_held());
    let mut cons = FrozenCons::new(&rb);
    assert_eq!(cons.try_pop(), Some(1));
}
