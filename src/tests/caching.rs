use super::Rb;
use crate::{CachingCons, CachingProd, Cons, Prod, storage::Array, traits::*, wrap::Wrap};
use core::{cell::Cell, mem};

#[test]
fn cached_endpoints_publish_and_fetch_progress() {
    let rb = Rb::<Array<i32, 2>>::default();
    let mut prod = CachingProd::new(&rb);
    let mut cons = CachingCons::new(&rb);
    prod.push_slice(&[0, 1]);
    assert_eq!(cons.try_pop(), Some(0));
    prod.try_push(2).unwrap();
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
    fn consume_and_forget<C: Consumer>(mut cons: C) {
        drop(cons.try_pop().unwrap());
        mem::forget(cons);
    }
    for cached in [false, true] {
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
            if cached {
                consume_and_forget(CachingCons::new(&rb));
            } else {
                consume_and_forget(Cons::new(&rb));
            }
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
    let mut prod = CachingProd::new(&rb);
    prod.try_push(CountDrop(&drops)).ok().unwrap();
    mem::forget(prod);
    assert_eq!(rb.occupied_len(), 1);
    drop(rb);
    assert_eq!(drops.get(), 1);
}

#[test]
fn extraction_releases_endpoint_rights() {
    let rb = Rb::<Array<i32, 2>>::default();
    let prod = Prod::new(&rb);
    let cons = Cons::new(&rb);
    let obs = prod.observe();
    assert!(obs.write_is_held() && obs.read_is_held());
    let rb_ref = prod.into_rb_ref();
    assert!(!obs.write_is_held() && obs.read_is_held());
    let mut prod = CachingProd::new(rb_ref);
    prod.try_push(1).unwrap();
    drop(prod);
    drop(cons);
    assert!(!obs.write_is_held() && !obs.read_is_held());
    assert_eq!(CachingCons::new(&rb).try_pop(), Some(1));
}
