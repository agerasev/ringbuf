use crate::{LocalRb, SharedRb};

use test::{black_box, Bencher};

const RB_SIZE: usize = 256;

#[bench]
fn push_pop_shared(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.push(1).unwrap();
        black_box(cons.pop().unwrap());
    });
}

#[bench]
fn push_pop_local(b: &mut Bencher) {
    let buf = LocalRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.push(1).unwrap();
        black_box(cons.pop().unwrap());
    });
}
