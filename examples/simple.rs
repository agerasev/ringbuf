use ringbuf::{traits::*, HeapRb};

fn main() {
    let rb = HeapRb::<i32>::new(2);
    let (mut prod, mut cons) = rb.split();

    prod.try_push(0).unwrap();
    prod.try_push(1).unwrap();
    assert_eq!(prod.try_push(2), Err(2));

    assert_eq!(cons.try_pop().unwrap(), 0);

    prod.try_push(2).unwrap();

    assert_eq!(cons.try_pop().unwrap(), 1);
    assert_eq!(cons.try_pop().unwrap(), 2);
    assert_eq!(cons.try_pop(), None);
}
