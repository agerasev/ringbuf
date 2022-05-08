use crate::HeapRingBuffer;

use test::{black_box, Bencher};

const RB_SIZE: usize = 256;

#[bench]
fn acquire(b: &mut Bencher) {
    let buf = HeapRingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        black_box(prod.acquire());
        black_box(cons.acquire());
    });
}

#[bench]
fn acquire_advance(b: &mut Bencher) {
    let buf = HeapRingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        unsafe { prod.acquire().advance(1) };
        unsafe { cons.acquire().advance(1) };
    });
}

#[bench]
fn get_occupied_slices(b: &mut Bencher) {
    let buf = HeapRingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 3 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    let mut cons_l = cons.acquire();
    b.iter(|| {
        black_box(unsafe { cons_l.as_mut_uninit_slices() });
        black_box(&mut cons_l);
    });
}

#[bench]
fn get_vacant_slices(b: &mut Bencher) {
    let buf = HeapRingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[0; 1 * RB_SIZE / 4]);
    cons.skip(RB_SIZE);
    prod.push_slice(&[1; RB_SIZE / 2]);
    let mut prod_l = prod.acquire();
    b.iter(|| {
        black_box(unsafe { prod_l.free_space_as_slices() });
        black_box(&mut prod_l);
    });
}

#[bench]
fn push_pop(b: &mut Bencher) {
    let buf = HeapRingBuffer::<u64>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    b.iter(|| {
        prod.push(1).unwrap();
        black_box(cons.pop().unwrap());
    });
}
