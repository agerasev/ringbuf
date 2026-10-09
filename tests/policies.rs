#![cfg(feature = "alloc")]
use ringbuf::{
    Rb, SharedRb,
    endpoint::{self, CachedCons, CachedProd, DirectCons},
    indices::*,
    markers::*,
    rb::RbHandle,
    storage::Array,
    traits::*,
};
use std::{mem, rc::Rc};

#[test]
fn no_markers_split_and_exclusive_recovery() {
    let mut rb = Rb::<Array<i32, 2>, LocalIndices, NoMarkers>::default();
    {
        let (mut p, c) = rb.split_ref();
        p.try_push(7).unwrap();
        mem::forget(c);
    }
    let (_, mut c) = rb.split_ref();
    assert_eq!(c.try_pop(), Some(7));
    drop(c);
    assert!(matches!(DirectCons::try_new(&rb), Err((endpoint::AcquireError::Untracked, _))));
}

#[test]
fn failed_pair_acquisition_rolls_back_only_its_own_rights() {
    let rb = SharedRb::<Array<i32, 2>>::default();
    let c = CachedCons::new(&rb);
    assert!(matches!(endpoint::try_split(&rb), Err((endpoint::AcquireError::ConsumerHeld, _))));
    assert!(!rb.write_is_held());
    assert!(rb.read_is_held());
    let p = CachedProd::new(&rb);
    assert!(matches!(CachedProd::try_new(&rb), Err((endpoint::AcquireError::ProducerHeld, _))));
    drop((p, c));
    assert!(!rb.read_is_held());
    assert!(!rb.write_is_held());
}

#[derive(Clone)]
struct Handle(Rc<SharedRb<Array<i32, 2>>>);
impl AsRef<SharedRb<Array<i32, 2>>> for Handle {
    fn as_ref(&self) -> &SharedRb<Array<i32, 2>> {
        &self.0
    }
}
unsafe impl RbHandle for Handle {
    type Rb = SharedRb<Array<i32, 2>>;
}

#[test]
fn caller_supplied_handle_and_independent_policies() {
    let h = Handle(Rc::new(SharedRb::default()));
    let (mut p, mut c) = endpoint::try_split(h).ok().unwrap();
    p.try_push(42).unwrap();
    assert_eq!(c.try_pop(), Some(42));
    let rb = Rb::<Array<i32, 2>, AtomicIndices, NoMarkers>::default();
    let (mut p, mut c) = rb.split();
    std::thread::spawn(move || p.try_push(9).unwrap()).join().unwrap();
    assert_eq!(c.try_pop(), Some(9));
}
