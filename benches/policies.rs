#![feature(test)]
extern crate test;

use ringbuf::{
    CachedCons as CachingCons, CachedProd as CachingProd, DirectCons as Cons, DirectProd as Prod, LocalRb, SharedRb, storage::Array, traits::*,
};
use std::hint::black_box;
use test::Bencher;

fn pairs(b: &mut Bencher, mut prod: impl Producer<Item = u64>, mut cons: impl Consumer<Item = u64>) {
    prod.push_slice(&[1; 128]);
    b.iter(|| {
        prod.try_push(1).unwrap();
        black_box(cons.try_pop().unwrap());
    });
}

macro_rules! pair_bench {
    ($name:ident, $rb:ident, $prod:ident, $cons:ident, $cap:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let rb = $rb::<Array<u64, $cap>>::default();
            pairs(b, $prod::new(&rb), $cons::new(&rb));
        }
    };
}

pair_bench!(pair_local_direct_256, LocalRb, Prod, Cons, 256);
pair_bench!(pair_shared_direct_256, SharedRb, Prod, Cons, 256);
pair_bench!(pair_local_cached_256, LocalRb, CachingProd, CachingCons, 256);
pair_bench!(pair_shared_cached_256, SharedRb, CachingProd, CachingCons, 256);
pair_bench!(pair_local_direct_257, LocalRb, Prod, Cons, 257);
pair_bench!(pair_shared_direct_257, SharedRb, Prod, Cons, 257);
pair_bench!(pair_local_cached_257, LocalRb, CachingProd, CachingCons, 257);
pair_bench!(pair_shared_cached_257, SharedRb, CachingProd, CachingCons, 257);

fn slices(b: &mut Bencher, mut prod: impl Producer<Item = u64>, mut cons: impl Consumer<Item = u64>) {
    let mut data = [1; 100];
    prod.push_slice(&[1; 128]);
    b.iter(|| {
        assert_eq!(prod.push_slice(&data), data.len());
        assert_eq!(cons.pop_slice(&mut data), data.len());
        black_box(&data);
    });
}

#[bench]
fn slice100_local_cached(b: &mut Bencher) {
    let rb = LocalRb::<Array<u64, 256>>::default();
    slices(b, CachingProd::new(&rb), CachingCons::new(&rb));
}

#[bench]
fn slice100_shared_cached(b: &mut Bencher) {
    let rb = SharedRb::<Array<u64, 256>>::default();
    slices(b, CachingProd::new(&rb), CachingCons::new(&rb));
}
