use crate::{traits::*, HeapRb};
use test::{black_box, Bencher};

const RB_SIZE: usize = 64 * 1024;

#[bench]
fn skip_bytes(b: &mut Bencher) {
    let buf = HeapRb::<u8>::new(RB_SIZE);
    let (mut prod, mut cons) = buf.split();
    prod.push_slice(&[1; RB_SIZE / 2]);
    let data = [1; RB_SIZE / 2];
    b.iter(|| {
        prod.push_slice(&data);
        black_box(cons.skip(black_box(RB_SIZE / 2)));
    });
}
