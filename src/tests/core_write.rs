use crate::StaticRb;
use core::fmt::Write;

#[test]
fn write() {
    let mut buf = StaticRb::<u8, 80>::default();

    let (mut prod, mut cons) = buf.split_ref();

    assert_eq!(write!(prod, "Hello world!\n"), Ok(()));
    assert_eq!(write!(prod, "The answer is {}\n", 42), Ok(()));

    assert_eq!(cons.len(), 30);
    assert_eq!(cons.pop(), Some(b'H'));
    assert_eq!(cons.pop(), Some(b'e'));
    assert_eq!(cons.pop(), Some(b'l'));
    assert_eq!(cons.pop(), Some(b'l'));
    assert_eq!(cons.pop(), Some(b'o'));
}

#[test]
fn write_overflow() {
    let mut buf = StaticRb::<u8, 10>::default();

    let (mut prod, mut cons) = buf.split_ref();

    assert_eq!(
        write!(
            prod,
            "This is a very long string that will overflow the small buffer\n"
        ),
        Err(core::fmt::Error::default())
    );

    assert_eq!(cons.len(), 10);
    assert_eq!(cons.pop(), Some(b'T'));
    assert_eq!(cons.pop(), Some(b'h'));
    assert_eq!(cons.pop(), Some(b'i'));
    assert_eq!(cons.pop(), Some(b's'));
    assert_eq!(cons.pop(), Some(b' '));
    assert_eq!(cons.pop(), Some(b'i'));
    assert_eq!(cons.pop(), Some(b's'));
    assert_eq!(cons.pop(), Some(b' '));
    assert_eq!(cons.pop(), Some(b'a'));
    assert_eq!(cons.pop(), Some(b' '));
    assert_eq!(cons.pop(), None);

    assert_eq!(
        write!(
            prod,
            "{} {} {} {} {}\n",
            "This", "string", "will", "also", "overflow"
        ),
        Err(core::fmt::Error::default())
    );

    assert_eq!(cons.len(), 10);
    assert_eq!(cons.pop(), Some(b'T'));
    assert_eq!(cons.pop(), Some(b'h'));
    assert_eq!(cons.pop(), Some(b'i'));
    assert_eq!(cons.pop(), Some(b's'));
    assert_eq!(cons.pop(), Some(b' '));
    assert_eq!(cons.pop(), Some(b's'));
    assert_eq!(cons.pop(), Some(b't'));
    assert_eq!(cons.pop(), Some(b'r'));
    assert_eq!(cons.pop(), Some(b'i'));
    assert_eq!(cons.pop(), Some(b'n'));
    assert_eq!(cons.pop(), None);
}
