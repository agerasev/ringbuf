use crate::{storage::Static, traits::*, LocalRb, SharedRb};
use test::{black_box, Bencher};

const RB_SIZE: usize = 256;
const BATCH_SIZE: usize = 100;

#[bench]
fn push_pop_shared(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split_arc();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.try_push(1).unwrap();
        black_box(cons.try_pop().unwrap());
    });
}

#[bench]
fn push_pop_local(b: &mut Bencher) {
    let buf = LocalRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split_arc();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.try_push(1).unwrap();
        black_box(cons.try_pop().unwrap());
    });
}

#[bench]
fn push_pop_x100(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split_arc();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        for _ in 0..BATCH_SIZE {
            prod.try_push(1).unwrap();
        }
        for _ in 0..BATCH_SIZE {
            black_box(cons.try_pop().unwrap());
        }
    });
}

#[bench]
fn push_pop_x100_cached(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split_arc();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        {
            let mut prod_cache = prod.cached();
            for _ in 0..BATCH_SIZE {
                prod_cache.try_push(1).unwrap();
            }
        }
        {
            let mut cons_cache = cons.cached();
            for _ in 0..BATCH_SIZE {
                black_box(cons_cache.try_pop().unwrap());
            }
        }
    });
}
