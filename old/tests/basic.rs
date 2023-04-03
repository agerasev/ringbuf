use crate::HeapRb;
#[cfg(feature = "std")]
use std::thread;

#[cfg(feature = "std")]
#[test]
fn split_threads() {
    let buf = HeapRb::<i32>::new(10);
    let (prod, cons) = buf.split();

    let pjh = thread::spawn(move || {
        let _ = prod;
    });

    let cjh = thread::spawn(move || {
        let _ = cons;
    });

    pjh.join().unwrap();
    cjh.join().unwrap();
}
