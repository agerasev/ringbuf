use crate::HeapRb;
use core::mem::MaybeUninit;

#[test]
fn push() {
    let cap = 3;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(vs_20.0);
        left[1] = MaybeUninit::new(vs_20.1);
        unsafe { prod.advance(2) };
    }
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
    }
    assert_eq!(cons.pop().unwrap(), vs_20.0);
    assert_eq!(cons.pop().unwrap(), vs_20.1);
    assert_eq!(cons.pop(), None);

    let vs_11 = (123, 456);
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 2);
        left[0] = MaybeUninit::new(vs_11.0);
        right[0] = MaybeUninit::new(vs_11.1);
        unsafe { prod.advance(2) };
    }
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
    }
    assert_eq!(cons.pop().unwrap(), vs_11.0);
    assert_eq!(cons.pop().unwrap(), vs_11.1);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_full() {
    let cap = 2;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    for i in 0..cap {
        prod.push(i as i32).unwrap();
    }
    assert_eq!(prod.push(0), Err(0));

    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), cap);
        assert_eq!(right.len(), 0);
        for (i, x) in left.iter().enumerate() {
            assert_eq!(unsafe { x.assume_init() }, i as i32);
        }
        unsafe { cons.advance(cap) };
    }

    assert_eq!(cons.len(), 0);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_empty() {
    let cap = 2;
    let buf = HeapRb::<i32>::new(cap);
    let (_, mut cons) = buf.split();

    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 0);
        assert_eq!(right.len(), 0);
        unsafe { cons.advance(0) };
    }
}

#[test]
fn pop() {
    let cap = 3;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456, 789);
    assert_eq!(prod.push(vs_20.0), Ok(()));
    assert_eq!(prod.push(vs_20.1), Ok(()));
    assert_eq!(prod.push(vs_20.2), Ok(()));
    assert_eq!(prod.push(0), Err(0));
    assert_eq!(prod.len(), 3);
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 0);
        assert_eq!(unsafe { left[0].assume_init() }, vs_20.0);
        assert_eq!(unsafe { left[1].assume_init() }, vs_20.1);
        assert_eq!(unsafe { left[2].assume_init() }, vs_20.2);
        unsafe { cons.advance(2) };
    }
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
    }
    assert_eq!(prod.len(), 1);

    let vs_11 = (654, 321);
    assert_eq!(prod.push(vs_11.0), Ok(()));
    assert_eq!(prod.push(vs_11.1), Ok(()));
    assert_eq!(prod.push(0), Err(0));
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 2);
        assert_eq!(unsafe { left[0].assume_init() }, vs_20.2);
        assert_eq!(unsafe { right[0].assume_init() }, vs_11.0);
        assert_eq!(unsafe { right[1].assume_init() }, vs_11.1);
        unsafe { cons.advance(2) };
    }
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
    }
    assert_eq!(prod.len(), 1);
    assert_eq!(cons.pop(), Some(vs_11.1));
}

#[test]
fn push_return() {
    let cap = 2;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        unsafe { prod.advance(0) };
    }

    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(12);
        unsafe { prod.advance(1) };
    }

    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(34);
        unsafe { prod.advance(1) };
    }

    assert_eq!(cons.pop().unwrap(), 12);
    assert_eq!(cons.pop().unwrap(), 34);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_return() {
    let cap = 2;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    assert_eq!(prod.push(12), Ok(()));
    assert_eq!(prod.push(34), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        unsafe { cons.advance(0) };
    }

    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(unsafe { left[0].assume_init() }, 12);
        unsafe { cons.advance(1) };
    }

    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        assert_eq!(unsafe { left[0].assume_init() }, 34);
        unsafe { cons.advance(1) };
    }

    assert_eq!(prod.len(), 0);
}

#[test]
fn push_pop() {
    let cap = 3;
    let buf = HeapRb::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(vs_20.0);
        left[1] = MaybeUninit::new(vs_20.1);
        unsafe { prod.advance(2) };
    }
    assert_eq!(prod.len(), 2);
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(unsafe { left[0].assume_init() }, vs_20.0);
        assert_eq!(unsafe { left[1].assume_init() }, vs_20.1);
        unsafe { cons.advance(2) };
    }
    assert_eq!(prod.len(), 0);

    let vs_11 = (123, 456);
    {
        let (left, right) = unsafe { prod.free_space_as_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 2);
        left[0] = MaybeUninit::new(vs_11.0);
        right[0] = MaybeUninit::new(vs_11.1);
        unsafe { prod.advance(2) };
    }
    assert_eq!(prod.len(), 2);
    {
        let (left, right) = unsafe { cons.as_uninit_slices() };
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(unsafe { left[0].assume_init() }, vs_11.0);
        assert_eq!(unsafe { right[0].assume_init() }, vs_11.1);
        unsafe { cons.advance(2) };
    }
    assert_eq!(prod.len(), 0);
}
