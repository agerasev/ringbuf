use crate::{prelude::*, storage::Static, LocalRb};
use core::fmt::Write;

#[test]
fn write() {
    let mut rb = LocalRb::<Static<u8, 40>>::default();

    let (mut prod, mut cons) = (&mut rb).split();

    assert_eq!(write!(prod, "Hello world!\n"), Ok(()));
    assert_eq!(write!(prod, "The answer is {}\n", 42), Ok(()));

    assert_eq!(cons.occupied_len(), 30);
    assert!(cons
        .pop_iter()
        .eq(b"Hello world!\nThe answer is 42\n".iter().cloned()));
}

#[test]
fn write_overflow() {
    let mut rb = LocalRb::<Static<u8, 10>>::default();

    let (mut prod, mut cons) = (&mut rb).split();

    assert_eq!(
        write!(
            prod,
            "This is a very long string that will overflow the small buffer\n"
        ),
        Err(core::fmt::Error::default())
    );

    assert_eq!(cons.occupied_len(), 10);
    assert!(cons.pop_iter().eq(b"This is a ".iter().cloned()));

    assert_eq!(
        write!(
            prod,
            "{} {} {} {} {}\n",
            "This", "string", "will", "also", "overflow"
        ),
        Err(core::fmt::Error::default())
    );

    assert_eq!(cons.occupied_len(), 10);
    assert!(cons.pop_iter().eq(b"This strin".iter().cloned()));
}
