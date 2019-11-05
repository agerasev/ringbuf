use super::*;

use std::cell::{RefCell};
use std::thread;
use std::io::{self, Read, Write};
use std::time::{Duration};
use std::sync::atomic::Ordering;
use std::mem::MaybeUninit;
use std::collections::{HashSet};


fn head_tail<T>(rb: &RingBuffer<T>) -> (usize, usize) {
    (rb.head.load(Ordering::Acquire), rb.tail.load(Ordering::Acquire))
}

#[test]
fn capacity() {
    let cap = 13;
    let buf = RingBuffer::<i32>::new(cap);
    assert_eq!(buf.capacity(), cap);
}
#[test]
fn split_capacity() {
    let cap = 13;
    let buf = RingBuffer::<i32>::new(cap);
    let (prod, cons) = buf.split();

    assert_eq!(prod.capacity(), cap);
    assert_eq!(cons.capacity(), cap);
}

#[test]
fn split_threads() {
    let buf = RingBuffer::<i32>::new(10);
    let (prod, cons) = buf.split();

    let pjh = thread::spawn(move || {
        let _ = prod;
    });

    let cjh = thread::spawn(move || {
        let _ = cons;
    });

    pjh.join().unwrap();
    cjh.join().unwrap();
}

#[test]
fn push() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, _) = buf.split();


    assert_eq!(head_tail(&prod.rb), (0, 0));

    assert_eq!(prod.push(123), Ok(()));
    assert_eq!(head_tail(&prod.rb), (0, 1));

    assert_eq!(prod.push(234), Ok(()));
    assert_eq!(head_tail(&prod.rb), (0, 2));

    assert_eq!(prod.push(345), Err(345));
    assert_eq!(head_tail(&prod.rb), (0, 2));
}

#[test]
fn pop_empty() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();


    assert_eq!(head_tail(&cons.rb), (0, 0));

    assert_eq!(cons.pop(), None);
    assert_eq!(head_tail(&cons.rb), (0, 0));
}

#[test]
fn push_pop_one() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vcap = cap + 1;
    let values = [12, 34, 56, 78, 90];
    assert_eq!(head_tail(&cons.rb), (0, 0));

    for (i, v) in values.iter().enumerate() {
        assert_eq!(prod.push(*v), Ok(()));
        assert_eq!(head_tail(&cons.rb), (i % vcap, (i + 1) % vcap));

        assert_eq!(cons.pop().unwrap(), *v);
        assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));

        assert_eq!(cons.pop(), None);
        assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));
    }
}

#[test]
fn push_pop_all() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vcap = cap + 1;
    let values = [(12, 34, 13), (56, 78, 57), (90, 10, 91)];
    assert_eq!(head_tail(&cons.rb), (0, 0));

    for (i, v) in values.iter().enumerate() {
        assert_eq!(prod.push(v.0), Ok(()));
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 1) % vcap));

        assert_eq!(prod.push(v.1), Ok(()));
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));

        assert_eq!(prod.push(v.2).unwrap_err(), v.2);
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));


        assert_eq!(cons.pop().unwrap(), v.0);
        assert_eq!(head_tail(&cons.rb), ((cap*i + 1) % vcap, (cap*i + 2) % vcap));

        assert_eq!(cons.pop().unwrap(), v.1);
        assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));

        assert_eq!(cons.pop(), None);
        assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));
    }
}

#[test]
fn empty_full() {
    let buf = RingBuffer::<i32>::new(1);
    let (mut prod, cons) = buf.split();

    assert!(prod.is_empty());
    assert!(cons.is_empty());
    assert!(!prod.is_full());
    assert!(!cons.is_full());

    assert_eq!(prod.push(123), Ok(()));

    assert!(!prod.is_empty());
    assert!(!cons.is_empty());
    assert!(prod.is_full());
    assert!(cons.is_full());
}

#[test]
fn len_remaining() {
    let buf = RingBuffer::<i32>::new(2);
    let (mut prod, mut cons) = buf.split();

    assert_eq!(prod.len(), 0);
    assert_eq!(cons.len(), 0);
    assert_eq!(prod.remaining(), 2);
    assert_eq!(cons.remaining(), 2);

    assert_eq!(prod.push(123), Ok(()));

    assert_eq!(prod.len(), 1);
    assert_eq!(cons.len(), 1);
    assert_eq!(prod.remaining(), 1);
    assert_eq!(cons.remaining(), 1);

    assert_eq!(prod.push(456), Ok(()));

    assert_eq!(prod.len(), 2);
    assert_eq!(cons.len(), 2);
    assert_eq!(prod.remaining(), 0);
    assert_eq!(cons.remaining(), 0);

    assert_eq!(cons.pop(), Some(123));

    assert_eq!(prod.len(), 1);
    assert_eq!(cons.len(), 1);
    assert_eq!(prod.remaining(), 1);
    assert_eq!(cons.remaining(), 1);

    assert_eq!(cons.pop(), Some(456));

    assert_eq!(prod.len(), 0);
    assert_eq!(cons.len(), 0);
    assert_eq!(prod.remaining(), 2);
    assert_eq!(cons.remaining(), 2);

    // now head is at 2, so tail will be at 0. This caught an overflow error
    // when tail+1 < head because of the substraction of usize.
    assert_eq!(prod.push(789), Ok(()));

    assert_eq!(prod.len(), 1);
    assert_eq!(cons.len(), 1);
    assert_eq!(prod.remaining(), 1);
    assert_eq!(cons.remaining(), 1);
}

#[derive(Debug)]
struct Dropper<'a> {
    id: i32,
    set: &'a RefCell<HashSet<i32>>,
}

impl<'a> Dropper<'a> {
    fn new(set: &'a RefCell<HashSet<i32>>, id: i32) -> Self {
        if !set.borrow_mut().insert(id) {
            panic!("value {} already exists", id);
        }
        Self { set, id }
    }
}

impl<'a> Drop for Dropper<'a> {
    fn drop(&mut self) {
        if !self.set.borrow_mut().remove(&self.id) {
            panic!("value {} already removed", self.id);
        }
    }
}

#[test]
fn drop() {
    let set = RefCell::new(HashSet::new());

    let cap = 3;
    let buf = RingBuffer::new(cap);

    assert_eq!(set.borrow().len(), 0);

    {
        let (mut prod, mut cons) = buf.split();

        prod.push(Dropper::new(&set, 1)).unwrap();
        assert_eq!(set.borrow().len(), 1);
        prod.push(Dropper::new(&set, 2)).unwrap();
        assert_eq!(set.borrow().len(), 2);
        prod.push(Dropper::new(&set, 3)).unwrap();
        assert_eq!(set.borrow().len(), 3);

        cons.pop().unwrap();
        assert_eq!(set.borrow().len(), 2);
        cons.pop().unwrap();
        assert_eq!(set.borrow().len(), 1);

        prod.push(Dropper::new(&set, 4)).unwrap();
        assert_eq!(set.borrow().len(), 2);
    }

    assert_eq!(set.borrow().len(), 0);
}

#[test]
fn push_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    let push_fn_20 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(vs_20.0);
        left[1] = MaybeUninit::new(vs_20.1);
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }, 2);

    assert_eq!(cons.pop().unwrap(), vs_20.0);
    assert_eq!(cons.pop().unwrap(), vs_20.1);
    assert_eq!(cons.pop(), None);

    let vs_11 = (123, 456);
    let push_fn_11 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = MaybeUninit::new(vs_11.0);
        right[0] = MaybeUninit::new(vs_11.1);
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }, 2);

    assert_eq!(cons.pop().unwrap(), vs_11.0);
    assert_eq!(cons.pop().unwrap(), vs_11.1);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_access_full() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [MaybeUninit<i32>], _r: &mut [MaybeUninit<i32>]| -> usize {
        0
    };
    assert_eq!(unsafe { cons.pop_access(dummy_fn) }, 0);
}

#[test]
fn pop_access_empty() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [MaybeUninit<i32>], _r: &mut [MaybeUninit<i32>]| -> usize {
        0
    };
    assert_eq!(unsafe { cons.pop_access(dummy_fn) }, 0);
}

#[test]
fn pop_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();


    let vs_20 = (123, 456);

    assert_eq!(prod.push(vs_20.0), Ok(()));
    assert_eq!(prod.push(vs_20.1), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_20 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0].assume_init(), vs_20.0);
        assert_eq!(left[1].assume_init(), vs_20.1);
        2
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }, 2);


    let vs_11 = (123, 456);

    assert_eq!(prod.push(vs_11.0), Ok(()));
    assert_eq!(prod.push(vs_11.1), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_11 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left[0].assume_init(), vs_11.0);
        assert_eq!(right[0].assume_init(), vs_11.1);
        2
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }, 2);

}

#[test]
fn push_access_return() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let push_fn_0 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        0
    };

    assert_eq!(unsafe { prod.push_access(push_fn_0) }, 0);

    let push_fn_1 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(12);
        1
    };

    assert_eq!(unsafe { prod.push_access(push_fn_1) }, 1);

    let push_fn_2 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(34);
        1
    };

    assert_eq!(unsafe { prod.push_access(push_fn_2) }, 1);

    assert_eq!(cons.pop().unwrap(), 12);
    assert_eq!(cons.pop().unwrap(), 34);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_access_return() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    assert_eq!(prod.push(12), Ok(()));
    assert_eq!(prod.push(34), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_0 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        0
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_0) }, 0);

    let pop_fn_1 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0].assume_init(), 12);
        1
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_1) }, 1);

    let pop_fn_2 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0].assume_init(), 34);
        1
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_2) }, 1);
}

#[test]
fn push_pop_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    let push_fn_20 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = MaybeUninit::new(vs_20.0);
        left[1] = MaybeUninit::new(vs_20.1);
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }, 2);

    let pop_fn_20 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0].assume_init(), vs_20.0);
        assert_eq!(left[1].assume_init(), vs_20.1);
        2
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }, 2);


    let vs_11 = (123, 456);
    let push_fn_11 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = MaybeUninit::new(vs_11.0);
        right[0] = MaybeUninit::new(vs_11.1);
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }, 2);

    let pop_fn_11 = |left: &mut [MaybeUninit<i32>], right: &mut [MaybeUninit<i32>]| -> usize { unsafe {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left[0].assume_init(), vs_11.0);
        assert_eq!(right[0].assume_init(), vs_11.1);
        2
    }};

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }, 2);
}

#[test]
fn push_pop_slice() {
    let buf = RingBuffer::<i32>::new(4);
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
    let buf0 = RingBuffer::<i32>::new(4);
    let buf1 = RingBuffer::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(prod1.move_from(&mut cons0, None), 3);
    assert_eq!(prod1.move_from(&mut cons0, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);


    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    assert_eq!(prod1.move_from(&mut cons0, None), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);


    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    assert_eq!(prod1.move_from(&mut cons0, None), 1);
    assert_eq!(prod1.move_from(&mut cons0, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn move_slice_count() {
    let buf0 = RingBuffer::<i32>::new(4);
    let buf1 = RingBuffer::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(prod1.move_from(&mut cons0, Some(2)), 2);

    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [0, 1]);

    assert_eq!(prod1.move_from(&mut cons0, Some(2)), 1);

    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [2]);


    assert_eq!(prod0.push_slice(&[3, 4, 5, 6]), 4);

    assert_eq!(prod1.move_from(&mut cons0, Some(3)), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);

    assert_eq!(prod0.push_slice(&[7, 8, 9]), 3);

    assert_eq!(prod1.move_from(&mut cons0, Some(5)), 4);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn read_from() {
    let buf0 = RingBuffer::<u8>::new(4);
    let buf1 = RingBuffer::<u8>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, None) {
        Err(e) => {
            assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
        },
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);


    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [3, 4]);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [5]);


    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 0),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn write_into() {
    let buf0 = RingBuffer::<u8>::new(4);
    let buf1 = RingBuffer::<u8>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 0),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);


    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [3, 4]);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [5]);


    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, None) {
        Err(e) => {
            assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
        },
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn read_from_write_into_count() {
    let buf0 = RingBuffer::<u8>::new(4);
    let buf1 = RingBuffer::<u8>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2, 3]), 4);

    match prod1.read_from(&mut cons0, Some(3)) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, Some(2)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [0, 1, 2, 3]);


    assert_eq!(prod0.push_slice(&[4, 5, 6, 7]), 4);

    match cons0.write_into(&mut prod1, Some(3)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, Some(2)) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, Some(2)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [4, 5, 6, 7]);
}

const THE_BOOK_FOREWORD: &'static str = "
It wasn’t always so clear, but the Rust programming language is fundamentally about empowerment: no matter what kind of code you are writing now, Rust empowers you to reach farther, to program with confidence in a wider variety of domains than you did before.
Take, for example, “systems-level” work that deals with low-level details of memory management, data representation, and concurrency. Traditionally, this realm of programming is seen as arcane, accessible only to a select few who have devoted the necessary years learning to avoid its infamous pitfalls. And even those who practice it do so with caution, lest their code be open to exploits, crashes, or corruption.
Rust breaks down these barriers by eliminating the old pitfalls and providing a friendly, polished set of tools to help you along the way. Programmers who need to “dip down” into lower-level control can do so with Rust, without taking on the customary risk of crashes or security holes, and without having to learn the fine points of a fickle toolchain. Better yet, the language is designed to guide you naturally towards reliable code that is efficient in terms of speed and memory usage.
Programmers who are already working with low-level code can use Rust to raise their ambitions. For example, introducing parallelism in Rust is a relatively low-risk operation: the compiler will catch the classical mistakes for you. And you can tackle more aggressive optimizations in your code with the confidence that you won’t accidentally introduce crashes or vulnerabilities.
But Rust isn’t limited to low-level systems programming. It’s expressive and ergonomic enough to make CLI apps, web servers, and many other kinds of code quite pleasant to write — you’ll find simple examples of both later in the book. Working with Rust allows you to build skills that transfer from one domain to another; you can learn Rust by writing a web app, then apply those same skills to target your Raspberry Pi.
This book fully embraces the potential of Rust to empower its users. It’s a friendly and approachable text intended to help you level up not just your knowledge of Rust, but also your reach and confidence as a programmer in general. So dive in, get ready to learn—and welcome to the Rust community!

— Nicholas Matsakis and Aaron Turon
";

#[test]
fn push_pop_slice_message() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while bytes.len() > 0 {
            let n = prod.push_slice(bytes);
            if n > 0 {
                bytes = &bytes[n..bytes.len()]
            } else {
                thread::sleep(Duration::from_millis(1))
            }
        }
        loop {
            match prod.push(0) {
                Ok(()) => break,
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        loop {
            let n = cons.pop_slice(&mut buffer);
            if n > 0 {
                bytes.extend_from_slice(&buffer[0..n])
            } else {
                if bytes.ends_with(&[0]) {
                    break;
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}

#[test]
fn read_from_write_into_message() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let zero = [0 as u8];
        let mut bytes = smsg.as_bytes().chain(&zero[..]);
        loop {
            if prod.is_full() {
                thread::sleep(Duration::from_millis(1));
            } else {
                if prod.read_from(&mut bytes, None).unwrap() == 0 {
                    break;
                }
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        loop {
            if cons.is_empty() {
                if bytes.ends_with(&[0]) {
                    break;
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            } else {
                cons.write_into(&mut bytes, None).unwrap();
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}

#[test]
fn read_write_message() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = THE_BOOK_FOREWORD;

    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while bytes.len() > 0 {
            match prod.write(bytes) {
                Ok(n) => bytes = &bytes[n..bytes.len()],
                Err(err) => {
                    assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                    thread::sleep(Duration::from_millis(1));
                },
            }
        }
        loop {
            match prod.push(0) {
                Ok(()) => break,
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        loop {
            match cons.read(&mut buffer) {
                Ok(n) => bytes.extend_from_slice(&buffer[0..n]),
                Err(err) => {
                    assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                    if bytes.ends_with(&[0]) {
                        break;
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                },
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}
