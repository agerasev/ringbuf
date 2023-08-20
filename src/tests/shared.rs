use crate::{storage::Heap, traits::*, SharedRb};
use std::{thread, thread::sleep, time::Duration, vec::Vec};

#[cfg(feature = "std")]
#[test]
fn concurrent() {
    fn yield_() {
        sleep(Duration::from_millis(1));
    }

    const MSG: &[u8] = b"The quick brown fox jumps over the lazy dog\0";
    let rb = SharedRb::<Heap<u8>>::new(4);
    let (mut prod, mut cons) = rb.split();

    let pjh = thread::spawn({
        let mut msg = MSG;
        move || {
            while !msg.is_empty() {
                prod.read_from(&mut msg, None).transpose().unwrap();
                yield_();
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut msg = Vec::new();
        while msg.last().copied() != Some(0) {
            cons.write_into(&mut msg, None).transpose().unwrap();
            yield_();
        }
        msg
    });

    pjh.join().unwrap();
    assert_eq!(cjh.join().unwrap(), MSG);
}

#[cfg(feature = "std")]
#[test]
#[ignore]
fn concurrent_spin_many() {
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
            }
        }
    });

    pjh.join().unwrap();
    assert_eq!(cjh.join().unwrap(), COUNT);
}
