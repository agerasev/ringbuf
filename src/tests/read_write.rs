use crate::{storage::Static, traits::*, LocalRb};
use std::io;

#[test]
fn from() {
    let mut rb0 = LocalRb::<Static<u8, 4>>::default();
    let mut rb1 = LocalRb::<Static<u8, 4>>::default();
    let (mut prod0, mut cons0) = rb0.split_ref();
    let (mut prod1, mut cons1) = rb1.split_ref();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(prod1.read_from(&mut cons0, None).unwrap(), 3);
    assert_eq!(
        prod1.read_from(&mut cons0, None).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    assert_eq!(prod1.read_from(&mut cons0, None).unwrap(), 1);
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [3]);

    assert_eq!(prod1.read_from(&mut cons0, None).unwrap(), 2);
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [4, 5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    assert_eq!(prod1.read_from(&mut cons0, None).unwrap(), 1);
    assert_eq!(prod1.read_from(&mut cons0, None).unwrap(), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn into() {
    let mut rb0 = LocalRb::<Static<u8, 4>>::default();
    let mut rb1 = LocalRb::<Static<u8, 4>>::default();
    let (mut prod0, mut cons0) = rb0.split_ref();
    let (mut prod1, mut cons1) = rb1.split_ref();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(cons0.write_into(&mut prod1, None).unwrap(), 3);
    assert_eq!(cons0.write_into(&mut prod1, None).unwrap(), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    assert_eq!(cons0.write_into(&mut prod1, None).unwrap(), 1);
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [3]);

    assert_eq!(cons0.write_into(&mut prod1, None).unwrap(), 2);
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [4, 5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    assert_eq!(cons0.write_into(&mut prod1, None).unwrap(), 1);
    assert_eq!(
        cons0.write_into(&mut prod1, None).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn count() {
    let mut rb0 = LocalRb::<Static<u8, 4>>::default();
    let mut rb1 = LocalRb::<Static<u8, 4>>::default();
    let (mut prod0, mut cons0) = rb0.split_ref();
    let (mut prod1, mut cons1) = rb1.split_ref();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2, 3]), 4);

    assert_eq!(prod1.read_from(&mut cons0, Some(3)).unwrap(), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[4, 5, 6]), 3);

    assert_eq!(cons0.write_into(&mut prod1, Some(3)).unwrap(), 1);
    assert_eq!(cons0.write_into(&mut prod1, Some(2)).unwrap(), 2);
    assert_eq!(cons0.write_into(&mut prod1, Some(2)).unwrap(), 1);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [3, 4, 5, 6]);
}
