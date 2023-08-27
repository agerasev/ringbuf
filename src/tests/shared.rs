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
