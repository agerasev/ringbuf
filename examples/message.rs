extern crate ringbuf;

use std::thread;
use std::time::{Duration};

use ringbuf::{RingBuffer, PushError, PopError};


fn main() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = "The quick brown fox jumps over the lazy dog";
    
    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while bytes.len() > 0 {
            match prod.push_slice(bytes) {
                Ok(n) => bytes = &bytes[n..bytes.len()],
                Err(PushError::Full) => thread::sleep(Duration::from_millis(1)),
            }
        }
        loop {
            match prod.push(0) {
                Ok(()) => break,
                Err((PushError::Full, _)) => thread::sleep(Duration::from_millis(1)),
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        loop {
            match cons.pop_slice(&mut buffer) {
                Ok(n) => bytes.extend_from_slice(&buffer[0..n]),
                Err(PopError::Empty) => {
                    if bytes.ends_with(&[0]) {
                        break;
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}
