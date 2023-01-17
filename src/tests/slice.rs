use crate::{transfer, HeapRb, Rb};

#[test]
fn push_pop_slice() {
    let buf = HeapRb::<i32>::new(4);
    let (mut prod, mut cons) = buf.split();

    let mut tmp = [0; 5];

    assert_eq!(prod.push_slice(&[]), 0);
    assert_eq!(prod.push_slice(&[0, 1, 2]), 3);

    assert_eq!(cons.pop_slice(&mut tmp[0..2]), 2);
    assert_eq!(tmp[0..2], [0, 1]);

    assert_eq!(prod.push_slice(&[3, 4]), 2);
    assert_eq!(prod.push_slice(&[5, 6]), 1);

    assert_eq!(cons.pop_slice(&mut tmp[0..3]), 3);
    assert_eq!(tmp[0..3], [2, 3, 4]);

    assert_eq!(prod.push_slice(&[6, 7, 8, 9]), 3);

    assert_eq!(cons.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [5, 6, 7, 8]);
}

#[test]
fn move_slice() {
    let buf0 = HeapRb::<i32>::new(4);
    let buf1 = HeapRb::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(transfer(&mut cons0, &mut prod1, None), 3);
    assert_eq!(transfer(&mut cons0, &mut prod1, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    assert_eq!(transfer(&mut cons0, &mut prod1, None), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    assert_eq!(transfer(&mut cons0, &mut prod1, None), 1);
    assert_eq!(transfer(&mut cons0, &mut prod1, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn move_slice_count() {
    let buf0 = HeapRb::<i32>::new(4);
    let buf1 = HeapRb::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(transfer(&mut cons0, &mut prod1, Some(2)), 2);

    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [0, 1]);

    assert_eq!(transfer(&mut cons0, &mut prod1, Some(2)), 1);

    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5, 6]), 4);

    assert_eq!(transfer(&mut cons0, &mut prod1, Some(3)), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);

    assert_eq!(prod0.push_slice(&[7, 8, 9]), 3);

    assert_eq!(transfer(&mut cons0, &mut prod1, Some(5)), 4);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
#[should_panic]
fn push_slice_panic() {
    let mut rb = HeapRb::<i32>::new(2);
    rb.push_slice(&[1, 2, 3]);
}

#[test]
#[should_panic]
fn pop_slice_panic() {
    let mut rb = HeapRb::<i32>::new(2);
    rb.push(1).unwrap();
    let mut tmp = [0; 2];
    rb.pop_slice(&mut tmp);
}
