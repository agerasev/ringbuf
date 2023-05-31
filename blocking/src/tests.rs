use crate::{
    halves::{BlockingCons, BlockingProd},
    rb::BlockingRb,
    sync::StdSemaphore,
    traits::*,
};
use ringbuf::{storage::Heap, traits::*, CachedCons, CachedProd, SharedRb};
use std::{iter::once, sync::Arc, thread, time::Duration};

const THE_BOOK_FOREWORD: &str = r#"
It wasn't always so clear, but the Rust programming language is fundamentally about empowerment: no matter what kind of code you are writing now, Rust empowers you to reach farther, to program with confidence in a wider variety of domains than you did before.
Take, for example, "systems-level" work that deals with low-level details of memory management, data representation, and concurrency. Traditionally, this realm of programming is seen as arcane, accessible only to a select few who have devoted the necessary years learning to avoid its infamous pitfalls. And even those who practice it do so with caution, lest their code be open to exploits, crashes, or corruption.
Rust breaks down these barriers by eliminating the old pitfalls and providing a friendly, polished set of tools to help you along the way. Programmers who need to "dip down" into lower-level control can do so with Rust, without taking on the customary risk of crashes or security holes, and without having to learn the fine points of a fickle toolchain. Better yet, the language is designed to guide you naturally towards reliable code that is efficient in terms of speed and memory usage.
Programmers who are already working with low-level code can use Rust to raise their ambitions. For example, introducing parallelism in Rust is a relatively low-risk operation: the compiler will catch the classical mistakes for you. And you can tackle more aggressive optimizations in your code with the confidence that you won't accidentally introduce crashes or vulnerabilities.
But Rust isn't limited to low-level systems programming. It's expressive and ergonomic enough to make CLI apps, web servers, and many other kinds of code quite pleasant to write - you'll find simple examples of both later in the book. Working with Rust allows you to build skills that transfer from one domain to another; you can learn Rust by writing a web app, then apply those same skills to target your Raspberry Pi.
This book fully embraces the potential of Rust to empower its users. It's a friendly and approachable text intended to help you level up not just your knowledge of Rust, but also your reach and confidence as a programmer in general. So dive in, get ready to learn-and welcome to the Rust community!

- Nicholas Matsakis and Aaron Turon
"#;

const TIMEOUT: Option<Duration> = Some(Duration::from_millis(1000));

type Rb = BlockingRb<SharedRb<Heap<u8>>, StdSemaphore>;

fn make_and_split(cap: usize) -> (BlockingProd<CachedProd<Arc<Rb>>>, BlockingCons<CachedCons<Arc<Rb>>>) {
    let rb = Arc::new(Rb::new(SharedRb::new(cap)));
    unsafe {
        (
            BlockingProd::new(CachedProd::new(rb.clone())),
            BlockingCons::new(CachedCons::new(rb)),
        )
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn wait() {
    let (mut prod, mut cons) = make_and_split(7);

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while !bytes.is_empty() {
            assert!(prod.wait_vacant(1, TIMEOUT));
            let n = prod.push_slice(bytes);
            assert!(n > 0);
            bytes = &bytes[n..bytes.len()]
        }
        assert!(prod.wait_vacant(1, TIMEOUT));
        prod.try_push(0).unwrap();
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        loop {
            assert!(cons.wait_occupied(1, TIMEOUT));
            let n = cons.pop_slice(&mut buffer);
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[0..n]);
            if bytes.ends_with(&[0]) {
                break;
            }
        }
        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn slice_all() {
    let (mut prod, mut cons) = make_and_split(7);

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let bytes = smsg.as_bytes();
        assert_eq!(prod.push_slice_all(bytes, TIMEOUT), bytes.len());
        prod.push(0, TIMEOUT).unwrap();
    });

    let cjh = thread::spawn(move || {
        let mut bytes = vec![0u8; smsg.as_bytes().len()];
        assert_eq!(cons.pop_slice_all(&mut bytes, TIMEOUT), bytes.len());
        assert_eq!(cons.pop_wait(TIMEOUT).unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}

#[test]
#[cfg_attr(miri, ignore)]
fn iter_all() {
    let (mut prod, mut cons) = make_and_split(7);

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let bytes = smsg.as_bytes();
        assert_eq!(prod.push_iter_all(bytes.iter().copied().chain(once(0)), TIMEOUT), bytes.len() + 1);
    });

    let cjh = thread::spawn(move || {
        let bytes = cons.pop_iter_all(TIMEOUT).take_while(|x| *x != 0).collect::<Vec<_>>();
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}
