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
    let buf = HeapRb::<i32>::new(2);
    let (mut prod, mut cons) = buf.split();

    prod.push(10).unwrap();
    prod.push(20).unwrap();

    for (i, v) in cons.pop_iter().enumerate() {
        assert_eq!(10 * (i + 1) as i32, v);
    }
    assert!(prod.is_empty());
}
