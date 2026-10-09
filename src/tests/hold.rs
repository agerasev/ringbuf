use super::Rb;
use crate::{CachedCons, CachedProd, Obs, storage::Array, traits::*};

#[test]
fn split_and_drop() {
    let mut rb = Rb::<Array<i32, 2>>::default();
    let (prod, cons) = rb.split_ref();
    let obs = prod.observe();

    assert!(obs.write_is_held() && obs.read_is_held());

    drop(cons);
    assert!(obs.write_is_held() && !obs.read_is_held());

    drop(prod);
    assert!(!obs.write_is_held() && !obs.read_is_held());
}

#[test]
fn manually_hold_and_drop() {
    let rb = Rb::<Array<i32, 2>>::default();
    let obs = Obs::new(&rb);
    assert!(!obs.write_is_held() && !obs.read_is_held());

    let cons = CachedCons::new(&rb);
    assert!(!obs.write_is_held() && obs.read_is_held());

    let prod = CachedProd::new(&rb);
    assert!(obs.write_is_held() && obs.read_is_held());

    drop(cons);
    assert!(obs.write_is_held() && !obs.read_is_held());

    drop(prod);
    assert!(!obs.write_is_held() && !obs.read_is_held());
}

#[test]
#[should_panic]
fn hold_conflict() {
    let rb = Rb::<Array<i32, 2>>::default();
    let _prod = CachedProd::new(&rb);
    CachedProd::new(&rb);
}

#[allow(unused_imports)]
use crate::traits::Presence;
