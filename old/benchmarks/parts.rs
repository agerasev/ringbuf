use crate::SharedRb;

use test::{black_box, Bencher};

const RB_SIZE: usize = 256;

#[bench]
fn make_postponed(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(prod.postponed());
        black_box(cons.postponed());
    });
}

#[bench]
fn advance(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.advance(1) };
        unsafe { cons.advance(1) };
    });
}

#[bench]
fn advance_postponed(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.postponed().advance(1) };
        unsafe { cons.postponed().advance(1) };
    });
}

#[bench]
fn get_occupied_slices(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 3 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(unsafe { cons.as_mut_uninit_slices() });
        black_box(&mut cons);
    });
}

#[bench]
fn get_vacant_slices(b: &mut Bencher) {
    let buf = SharedRb::<u64, [_; RB_SIZE]>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 1 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(unsafe { prod.free_space_as_slices() });
        black_box(&mut prod);
    });
}
