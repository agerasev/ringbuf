use crate::HeapRb;

#[test]
fn iter() {
    let buf = HeapRb::<i32>::new(2);
    let (mut prod, mut cons) = buf.split();

    prod.push(10).unwrap();
    prod.push(20).unwrap();

    let sum: i32 = cons.iter().sum();

    let first = cons.pop().expect("First item is not available");
    let second = cons.pop().expect("Second item is not available");

    assert_eq!(sum, first + second);
}

#[test]
fn iter_mut() {
    let buf = HeapRb::<i32>::new(2);
    let (mut prod, mut cons) = buf.split();

    prod.push(10).unwrap();
    prod.push(20).unwrap();

    for v in cons.iter_mut() {
        *v *= 2;
    }

    let sum: i32 = cons.iter().sum();

    let first = cons.pop().expect("First item is not available");
    let second = cons.pop().expect("Second item is not available");

    assert_eq!(sum, first + second);
}

#[test]
fn pop_iter() {
    let buf = HeapRb::<i32>::new(3);
    let (mut prod, mut cons) = buf.split();

    prod.push(0).unwrap();
    prod.push(1).unwrap();
    for (i, v) in cons.pop_iter().enumerate() {
        assert_eq!(i as i32, v);
    }

    prod.push(2).unwrap();
    prod.push(3).unwrap();
    for (i, v) in cons.pop_iter().enumerate() {
        assert_eq!(i as i32 + 2, v);
    }
    assert!(prod.is_empty());
}

#[test]
fn push_pop_iter_partial() {
    let buf = HeapRb::<i32>::new(4);
    let (mut prod, mut cons) = buf.split();

    prod.push(0).unwrap();
    prod.push(1).unwrap();
    prod.push(2).unwrap();
    for (i, v) in (0..2).zip(cons.pop_iter()) {
        assert_eq!(i, v);
    }

    prod.push(3).unwrap();
    prod.push(4).unwrap();
    prod.push(5).unwrap();
    for (i, v) in (2..5).zip(cons.pop_iter()) {
        assert_eq!(i, v);
    }
    assert_eq!(cons.pop().unwrap(), 5);
    assert!(prod.is_empty());
}
