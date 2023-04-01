use crate::StaticRb;
use core::fmt::Write;

#[test]
fn write() {
    let mut buf = StaticRb::<u8, 40>::default();

    let (mut prod, mut cons) = buf.split_ref();

    assert_eq!(write!(prod, "Hello world!\n"), Ok(()));
    assert_eq!(write!(prod, "The answer is {}\n", 42), Ok(()));

    assert_eq!(cons.len(), 30);
    assert!(cons
        .pop_iter()
        .eq(b"Hello world!\nThe answer is 42\n".iter().cloned()));
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
    assert!(cons.pop_iter().eq(b"This is a ".iter().cloned()));

    assert_eq!(
        write!(
            prod,
            "{} {} {} {} {}\n",
            "This", "string", "will", "also", "overflow"
        ),
        Err(core::fmt::Error::default())
    );

    assert_eq!(cons.len(), 10);
    assert!(cons.pop_iter().eq(b"This strin".iter().cloned()));
}
