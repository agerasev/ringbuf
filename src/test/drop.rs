use std::{cell::RefCell, collections::HashSet};

use crate::RingBuffer;

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
fn single() {
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
fn multiple_fn() {
    let set = RefCell::new(HashSet::new());

    let cap = 5;
    let buf = RingBuffer::new(cap);

    assert_eq!(set.borrow().len(), 0);

    {
        let (mut prod, mut cons) = buf.split();
        let mut id = 0;
        let mut cnt = 0;

        assert_eq!(
            prod.push_fn(|| {
                if cnt < 4 {
                    id += 1;
                    cnt += 1;
                    Some(Dropper::new(&set, id))
                } else {
                    None
                }
            }),
            4
        );
        assert_eq!(cnt, 4);
        assert_eq!(cnt, set.borrow().len());

        assert_eq!(
            cons.pop_fn(
                |_| {
                    cnt -= 1;
                    true
                },
                Some(2)
            ),
            2
        );
        assert_eq!(cnt, 2);
        assert_eq!(cnt, set.borrow().len());

        assert_eq!(
            prod.push_fn(|| {
                id += 1;
                cnt += 1;
                Some(Dropper::new(&set, id))
            }),
            3
        );
        assert_eq!(cnt, 5);
        assert_eq!(cnt, set.borrow().len());

        assert_eq!(
            cons.pop_fn(
                |_| {
                    cnt -= 1;
                    true
                },
                None
            ),
            5
        );
        assert_eq!(cnt, 0);
        assert_eq!(cnt, set.borrow().len());

        assert_eq!(
            prod.push_fn(|| {
                id += 1;
                cnt += 1;
                Some(Dropper::new(&set, id))
            }),
            5
        );
        assert_eq!(cnt, 5);
        assert_eq!(cnt, set.borrow().len());
    }

    assert_eq!(set.borrow().len(), 0);
}
