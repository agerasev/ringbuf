use crate::{HeapRb, Rb};
use alloc::rc::Rc;

#[test]
fn skip() {
    // Initialize ringbuffer, prod and cons
    let rb = HeapRb::<i8>::new(10);
    let (mut prod, mut cons) = rb.split();
    let mut i = 0;

    // Fill the buffer
    for _ in 0..10 {
        prod.push(i).unwrap();
        i += 1;
    }

    // Pop in the middle of the buffer
    assert_eq!(cons.skip(5), 5);

    // Make sure changes are taken into account
    assert_eq!(cons.pop().unwrap(), 5);

    // Fill the buffer again
    for _ in 0..5 {
        prod.push(i).unwrap();
        i += 1;
    }

    assert_eq!(cons.skip(6), 6);
    assert_eq!(cons.pop().unwrap(), 12);

    // Fill the buffer again
    for _ in 0..7 {
        prod.push(i).unwrap();
        i += 1;
    }

    // Ask too much, delete the max number of items
    assert_eq!(cons.skip(10), 9);

    // Try to remove more than possible
    assert_eq!(cons.skip(1), 0);

    // Make sure it is still usable
    assert_eq!(cons.pop(), None);
    assert_eq!(prod.push(0), Ok(()));
    assert_eq!(cons.pop(), Some(0));
}

#[test]
fn skip_drop() {
    let rc = Rc::<()>::new(());

    static N: usize = 10;

    let rb = HeapRb::<Rc<()>>::new(N);
    let (mut prod, mut cons) = rb.split();

    for _ in 0..N {
        prod.push(rc.clone()).unwrap();
    }

    assert_eq!(cons.len(), N);
    assert_eq!(Rc::strong_count(&rc), N + 1);

    assert_eq!(cons.skip(N), N);

    // Check ring buffer is empty
    assert_eq!(cons.len(), 0);
    // Check that items are dropped
    assert_eq!(Rc::strong_count(&rc), 1);
}

#[test]
#[should_panic]
fn skip_panic() {
    let mut rb = HeapRb::<i32>::new(2);
    rb.push(1).unwrap();
    rb.skip(2);
}
