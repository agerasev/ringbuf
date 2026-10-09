use ringbuf::{ArrayRb, endpoint::Endpoint, traits::*};
use std::{
    cell::RefCell,
    mem,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn commit_fetch_undo_and_drop_across_wraparound() {
    let mut rb = ArrayRb::<i32, 3>::default();
    let (mut p, mut c) = rb.split_ref();
    for cycle in 0..12 {
        {
            let mut d = p.defer();
            assert_eq!(d.push_slice(&[cycle, 10, 20]), 3);
            assert!(c.try_pop().is_none());
            assert_eq!(d.undo_push(), Some(20));
            d.commit();
            assert!(d.undo_push().is_none());
            assert_eq!(c.try_pop(), Some(cycle));
            assert_eq!(d.fetch(), 1);
            d.try_push(30).unwrap();
        }
        {
            let mut d = c.defer();
            assert_eq!(d.try_pop(), Some(10));
            assert_eq!(d.undo_pop(11), Ok(()));
            assert_eq!(d.try_pop(), Some(11));
            assert_eq!(p.vacant_len(), 1); // no release yet
            d.commit();
            assert_eq!(d.undo_pop(99), Err(99));
            assert_eq!(p.vacant_len(), 2);
            // Leaving 30 unread returns it to the backing RB.
        }
        assert_eq!(c.try_pop(), Some(30));
        assert!(c.try_pop().is_none());
    }
}

#[test]
fn fetch_extends_claim_without_committing_and_lengths_distinguish_ownership() {
    let mut rb = ArrayRb::<i32, 4>::default();
    let (mut p, mut c) = rb.split_ref();
    let obs = p.observe();
    p.push_slice(&[1, 2]);
    let mut d = c.defer();
    assert_eq!((obs.retained_len(), obs.queued_len()), (2, 0));
    assert_eq!(d.try_pop(), Some(1));
    p.try_push(3).unwrap();
    assert_eq!((obs.retained_len(), obs.queued_len()), (3, 1));
    assert_eq!(d.fetch(), 1);
    assert_eq!((obs.retained_len(), obs.queued_len()), (3, 0));
    assert_eq!(d.pop_iter().collect::<Vec<_>>(), [2, 3]);
    assert_eq!(obs.retained_len(), 3);
    drop(d);
    assert_eq!((obs.retained_len(), obs.queued_len()), (0, 0));
}

struct Value {
    id: usize,
    drops: Rc<RefCell<Vec<usize>>>,
    panic: bool,
}
impl Drop for Value {
    fn drop(&mut self) {
        self.drops.borrow_mut().push(self.id);
        if self.panic {
            panic!("test destructor");
        }
    }
}
fn value(id: usize, drops: &Rc<RefCell<Vec<usize>>>) -> Value {
    Value {
        id,
        drops: drops.clone(),
        panic: false,
    }
}

#[test]
fn forgotten_borrowed_reads_leak_only_the_claim_and_resume_safely() {
    let drops = Rc::new(RefCell::new(vec![]));
    {
        let mut rb = ArrayRb::<Value, 4>::default();
        let (mut p, mut c) = rb.split_ref();
        p.push_iter((0..3).map(|i| value(i, &drops)));
        let mut d = c.defer();
        drop(d.try_pop().unwrap());
        mem::forget(d);
        p.try_push(value(3, &drops)).ok().unwrap();
        assert_eq!(c.try_pop().unwrap().id, 3);
        assert!(c.try_pop().is_none());
        p.try_push(value(4, &drops)).ok().unwrap();
        drop(c.try_pop());
    }
    assert_eq!(*drops.borrow(), [0, 3, 4]); // 1 and 2 deliberately leaked
}

#[test]
fn forgotten_owned_reads_are_not_dropped_by_the_rb() {
    let drops = Rc::new(RefCell::new(vec![]));
    {
        let mut rb = ArrayRb::<Value, 3>::default();
        let (mut p, c) = rb.split_ref();
        p.push_iter((0..3).map(|i| value(i, &drops)));
        let mut d = c.into_deferred();
        drop(d.try_pop());
        mem::forget(d);
    }
    assert_eq!(*drops.borrow(), [0]);
}

#[test]
fn forgotten_borrowed_writes_do_not_publish_and_cached_writer_recovers() {
    let drops = Rc::new(RefCell::new(vec![]));
    {
        let mut rb = ArrayRb::<Value, 3>::default();
        let (mut p, mut c) = rb.split_ref();
        let mut d = p.defer();
        d.try_push(value(0, &drops)).ok().unwrap();
        d.commit();
        d.try_push(value(1, &drops)).ok().unwrap();
        mem::forget(d);
        p.try_push(value(2, &drops)).ok().unwrap();
        assert_eq!(c.pop_iter().map(|v| v.id).collect::<Vec<_>>(), [0, 2]);
    }
    assert_eq!(*drops.borrow(), [0, 2]);
}

#[test]
fn discard_survives_a_panicking_destructor_without_publication() {
    let drops = Rc::new(RefCell::new(vec![]));
    let mut rb = ArrayRb::<Value, 3>::default();
    let (mut p, mut c) = rb.split_ref();
    let mut d = p.defer();
    d.push_iter((0..3).map(|i| {
        let mut v = value(i, &drops);
        v.panic = i == 1;
        v
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| d.discard())).is_err());
    assert_eq!(*drops.borrow(), [0, 1, 2]);
    assert_eq!(d.pending_len(), 0);
    d.try_push(value(3, &drops)).ok().unwrap();
    drop(d);
    assert_eq!(c.try_pop().unwrap().id, 3);
    assert!(c.try_pop().is_none());
}

#[test]
fn unwinding_commits_and_into_inner_restores_immediate_operation() {
    let mut rb = ArrayRb::<i32, 3>::default();
    let (mut p, mut c) = rb.split_ref();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let mut d = p.defer();
            d.try_push(1).unwrap();
            panic!("caller");
        }))
        .is_err()
    );
    assert_eq!(c.try_pop(), Some(1));
    let mut d = p.into_deferred();
    d.try_push(2).unwrap();
    let mut p = d.into_inner();
    p.try_push(3).unwrap();
    assert_eq!(c.pop_iter().collect::<Vec<_>>(), [2, 3]);
    let _handle = p.into_rb_handle();
}
