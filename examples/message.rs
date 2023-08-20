use ringbuf::{traits::*, HeapRb};
use std::{io::Read, thread, time::Duration};

fn main() {
    let buf = HeapRb::<u8>::new(10);
    let (mut prod, mut cons) = buf.split();

    let smsg = "The quick brown fox jumps over the lazy dog";

    let pjh = thread::spawn(move || {
        println!("-> sending message: '{}'", smsg);

        let zero = [0];
        let mut bytes = smsg.as_bytes().chain(&zero[..]);
        loop {
            if prod.is_full() {
                println!("-> buffer is full, waiting");
                thread::sleep(Duration::from_millis(1));
            } else {
                match prod.read_from(&mut bytes, None).transpose().unwrap() {
                    None | Some(0) => break,
                    Some(n) => println!("-> {} bytes sent", n),
                }
            }
        }

        println!("-> message sent");
    });

    let cjh = thread::spawn(move || {
        println!("<- receiving message");

        let mut bytes = Vec::<u8>::new();
        loop {
            if cons.is_empty() {
                if bytes.ends_with(&[0]) {
                    break;
                } else {
                    println!("<- buffer is empty, waiting");
                    thread::sleep(Duration::from_millis(1));
                }
            } else {
                let n = cons.write_into(&mut bytes, None).unwrap().unwrap();
                println!("<- {} bytes received", n);
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        let msg = String::from_utf8(bytes).unwrap();
        println!("<- message received: '{}'", msg);

        msg
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}
