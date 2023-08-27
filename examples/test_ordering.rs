//! This example checks atomic operations ordering on weak-ordered architectures (e.g. aarch64).
//! If there are some bugs with ordering the test *may* fail.

use ringbuf::{storage::Heap, traits::*, SharedRb};
use std::thread;

fn main() {
    const COUNT: usize = 10_000_000;
    let mut msg = (1..=u8::MAX).cycle().take(COUNT).chain([0]);

    let rb = SharedRb::<Heap<u8>>::new(17);
    let (mut prod, mut cons) = rb.split();

    let pjh = thread::spawn(move || {
        #[allow(unused_variables)]
        let mut i = 0;
        let mut y = msg.next();
        while let Some(x) = y {
            if prod.try_push(x).is_ok() {
                y = msg.next();
                i += 1;
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut i = 0;
        let mut y = 1;
        loop {
            if let Some(x) = cons.try_pop() {
                if x == 0 {
                    break i;
                }
                assert_eq!(x, y);
                i += 1;
                y = y.wrapping_add(1);
                if y == 0 {
                    y += 1;
                }
                if (i + 1) % (COUNT / 10) == 0 {
                    println!("... {}%", (i + 1) / (COUNT / 10) * 10);
                }
            }
        }
    });

    pjh.join().unwrap();
    assert_eq!(cjh.join().unwrap(), COUNT);

    println!("Success!");
}
