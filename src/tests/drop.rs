use crate::{Consumer, HeapRb};
use alloc::collections::BTreeSet;
use core::cell::RefCell;

#[derive(Debug)]
struct Dropper<'a> {
    id: i32,
    set: &'a RefCell<BTreeSet<i32>>,
}

impl<'a> Dropper<'a> {
    fn new(set: &'a RefCell<BTreeSet<i32>>, id: i32) -> Self {
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
    let set = RefCell::new(BTreeSet::new());

    let cap = 3;
    let buf = HeapRb::new(cap);

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

// TODO: Use transactions.
#[test]
fn transaction() {
    let set = RefCell::new(BTreeSet::new());

    let cap = 5;
    let buf = HeapRb::new(cap);

    assert_eq!(set.borrow().len(), 0);
    {
        let (mut prod, mut cons) = buf.split();
        let mut id = 0;
        let mut cnt = 0;
        let assert_cnt = |cnt, n, cons: &Consumer<_, _>, set: &RefCell<BTreeSet<_>>| {
            assert_eq!(cnt, n);
            assert_eq!(cnt, cons.len());
            assert_eq!(cnt, set.borrow().len());
        };

        for _ in 0..4 {
            id += 1;
            cnt += 1;
            prod.push(Dropper::new(&set, id)).unwrap();
        }
        assert_cnt(cnt, 4, &cons, &set);

        for _ in cons.pop_iter().take(2) {
            cnt -= 1;
        }
        assert_cnt(cnt, 2, &cons, &set);

        while !prod.is_full() {
            id += 1;
            cnt += 1;
            prod.push(Dropper::new(&set, id)).unwrap();
        }
        assert_cnt(cnt, 5, &cons, &set);

        for _ in cons.pop_iter() {
            cnt -= 1;
        }
        assert_cnt(cnt, 0, &cons, &set);

        while !prod.is_full() {
            id += 1;
            cnt += 1;
            prod.push(Dropper::new(&set, id)).unwrap();
        }
        assert_cnt(cnt, 5, &cons, &set);
    }

    assert_eq!(set.borrow().len(), 0);
}
