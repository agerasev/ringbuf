use crate::{traits::*, wrap::WaitError, BlockingHeapRb};
use std::{
    io::{Read, Write},
    sync::Arc,
    thread,
    time::Duration,
    vec,
    vec::Vec,
};

const THE_BOOK_FOREWORD: &[u8] = b"
It wasn't always so clear, but the Rust programming language is fundamentally about empowerment: no matter what kind of code you are writing now, Rust empowers you to reach farther, to program with confidence in a wider variety of domains than you did before.
Take, for example, \"systems-level\" work that deals with low-level details of memory management, data representation, and concurrency. Traditionally, this realm of programming is seen as arcane, accessible only to a select few who have devoted the necessary years learning to avoid its infamous pitfalls. And even those who practice it do so with caution, lest their code be open to exploits, crashes, or corruption.
Rust breaks down these barriers by eliminating the old pitfalls and providing a friendly, polished set of tools to help you along the way. Programmers who need to \"dip down\" into lower-level control can do so with Rust, without taking on the customary risk of crashes or security holes, and without having to learn the fine points of a fickle toolchain. Better yet, the language is designed to guide you naturally towards reliable code that is efficient in terms of speed and memory usage.
Programmers who are already working with low-level code can use Rust to raise their ambitions. For example, introducing parallelism in Rust is a relatively low-risk operation: the compiler will catch the classical mistakes for you. And you can tackle more aggressive optimizations in your code with the confidence that you won't accidentally introduce crashes or vulnerabilities.
But Rust isn't limited to low-level systems programming. It's expressive and ergonomic enough to make CLI apps, web servers, and many other kinds of code quite pleasant to write - you'll find simple examples of both later in the book. Working with Rust allows you to build skills that transfer from one domain to another; you can learn Rust by writing a web app, then apply those same skills to target your Raspberry Pi.
This book fully embraces the potential of Rust to empower its users. It's a friendly and approachable text intended to help you level up not just your knowledge of Rust, but also your reach and confidence as a programmer in general. So dive in, get ready to learn-and welcome to the Rust community!

- Nicholas Matsakis and Aaron Turon
";
const N_REP: usize = 10;

const TIMEOUT: Option<Duration> = Some(Duration::from_millis(1000));

#[test]
#[cfg_attr(miri, ignore)]
fn wait() {
    let rb = BlockingHeapRb::<u8>::new(7);
    let (mut prod, mut cons) = rb.split();

    let smsg = Arc::new(THE_BOOK_FOREWORD.repeat(N_REP));

    let pjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            let mut bytes = smsg.as_slice();
            prod.set_timeout(TIMEOUT);
            while !bytes.is_empty() {
                assert_eq!(prod.wait_vacant(1), Ok(()));
                let n = prod.push_slice(bytes);
                assert!(n > 0);
                bytes = &bytes[n..bytes.len()]
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        cons.set_timeout(TIMEOUT);
        loop {
            let res = cons.wait_occupied(1);
            if let Err(WaitError::Closed) = res {
                break;
            }
            assert_eq!(res, Ok(()));
            let n = cons.pop_slice(&mut buffer);
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[0..n]);
        }
        bytes
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(*smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn slice_all() {
    let rb = BlockingHeapRb::<u8>::new(7);
    let (mut prod, mut cons) = rb.split();

    let smsg = Arc::new(THE_BOOK_FOREWORD.repeat(N_REP));

    let pjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            let bytes = smsg;
            prod.set_timeout(TIMEOUT);
            assert_eq!(prod.push_exact(&bytes), bytes.len());
        }
    });

    let cjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            let mut bytes = vec![0u8; smsg.len()];
            cons.set_timeout(TIMEOUT);
            assert_eq!(cons.pop_exact(&mut bytes), bytes.len());
            bytes
        }
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(*smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn vec_all() {
    let rb = BlockingHeapRb::<u8>::new(7);
    let (mut prod, mut cons) = rb.split();

    let smsg = Arc::new(THE_BOOK_FOREWORD.repeat(N_REP));

    let pjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            let bytes = smsg;
            prod.set_timeout(TIMEOUT);
            assert_eq!(prod.push_exact(&bytes), bytes.len());
        }
    });

    let cjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            let mut bytes = Vec::new();
            cons.set_timeout(TIMEOUT);
            cons.pop_until_end(&mut bytes);
            assert_eq!(bytes.len(), smsg.len());
            bytes
        }
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(*smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn iter_all() {
    let rb = BlockingHeapRb::<u8>::new(7);
    let (mut prod, mut cons) = rb.split();

    let smsg = Arc::new(THE_BOOK_FOREWORD.repeat(N_REP));

    let pjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            prod.set_timeout(TIMEOUT);
            let bytes = smsg;
            assert_eq!(prod.push_all_iter(bytes.iter().copied()), bytes.len());
        }
    });

    let cjh = thread::spawn(move || {
        cons.set_timeout(TIMEOUT);
        cons.pop_all_iter().collect::<Vec<_>>()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(*smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn write_read() {
    let rb = BlockingHeapRb::<u8>::new(7);
    let (mut prod, mut cons) = rb.split();

    let smsg = Arc::new(THE_BOOK_FOREWORD.repeat(N_REP));

    let pjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            prod.set_timeout(TIMEOUT);
            let bytes = smsg;
            prod.write_all(&bytes).unwrap();
        }
    });

    let cjh = thread::spawn({
        let smsg = smsg.clone();
        move || {
            cons.set_timeout(TIMEOUT);
            let mut bytes = Vec::new();
            assert_eq!(cons.read_to_end(&mut bytes).unwrap(), smsg.len());
            bytes
        }
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(*smsg, rmsg);
}
