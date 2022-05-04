use crate::RingBuffer;

use test::{black_box, Bencher};

const RB_SIZE: usize = 0x100;

#[bench]
fn get_occupied_slices(b: &mut Bencher) {
    let buf = RingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 3 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(unsafe { cons.as_mut_uninit_slices() });
    });
}

#[bench]
fn get_vacant_slices(b: &mut Bencher) {
    let buf = RingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 1 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(unsafe { prod.free_space_as_slices() });
    });
}

#[bench]
fn advance(b: &mut Bencher) {
    let buf = RingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.advance(1) };
        unsafe { cons.advance(1) };
    });
}

#[bench]
fn push_pop(b: &mut Bencher) {
    let buf = RingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.push(1).unwrap();
        cons.pop().unwrap();
    });
}
