use crate::{storage::Static, traits::*, SharedRb};
use test::{black_box, Bencher};

const RB_SIZE: usize = 256;

#[bench]
fn make_postponed(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(prod.cached());
        black_box(cons.cached());
    });
}

#[bench]
fn advance(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.advance_write(1) };
        unsafe { cons.advance_read(1) };
    });
}

#[bench]
fn advance_postponed(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.cached().advance_write(1) };
        unsafe { cons.cached().advance_read(1) };
    });
}

#[bench]
fn get_occupied_slices(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 3 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(unsafe { cons.occupied_slices_mut() });
        black_box(&mut cons);
    });
}

#[bench]
fn get_vacant_slices(b: &mut Bencher) {
    let buf = SharedRb::<Static<u64, RB_SIZE>>::default();
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 1 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(prod.vacant_slices_mut());
        black_box(&mut prod);
    });
}
