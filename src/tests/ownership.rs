use super::Rb;
use crate::{
    StaticRb,
    rb::{Exclusive, RbMut},
    storage::Array,
    traits::*,
    wrap::{Cons, Obs, Prod, Wrap},
};
use core::cell::Cell;

#[test]
fn endpoint_views_keep_shared_receivers() {
    fn views<C: Consumer<Item = u8>>(consumer: &C) {
        assert_eq!(consumer.first(), Some(&1));
        assert_eq!(consumer.last(), Some(&2));
        assert_eq!(consumer.try_peek(), Some(&1));
        assert_eq!(consumer.iter().copied().sum::<u8>(), 3);
        assert_eq!(consumer.as_slices(), (&[1, 2][..], &[][..]));
        let mut copy = [0; 2];
        assert_eq!(consumer.peek_slice(&mut copy), 2);
        assert_eq!(copy, [1, 2]);
    }
    let mut rb = Rb::<Array<u8, 3>>::default();
    let (mut producer, mut consumer) = rb.split_ref();
    producer.push_slice(&[1, 2]);
    views(&consumer);
    let first = consumer.first().unwrap();
    producer.try_push(3).unwrap();
    assert_eq!(*first, 1);
    assert_eq!(consumer.try_pop(), Some(1));
}

#[test]
fn standalone_views_and_generic_adapter() {
    let mut rb = Rb::<Array<u8, 3>>::default();
    rb.push_slice(&[1, 2, 3]);
    assert_eq!(rb.try_pop(), Some(1));
    rb.try_push(4).unwrap();
    assert_eq!(rb.as_slices(), (&[2, 3][..], &[4][..]));
    assert_eq!(rb.first(), Some(&2));
    assert_eq!(rb.last(), Some(&4));
    for item in rb.iter_mut() {
        *item += 1;
    }
    assert_eq!(rb.iter().copied().sum::<u8>(), 12);
    let mut dst = Rb::<Array<u8, 3>>::default();
    assert_eq!(crate::transfer(&mut rb.as_exclusive(), &mut dst.as_exclusive(), None), 3);
    assert!(rb.is_empty());
    assert!(dst.pop_iter().eq([3, 4, 5]));
}

#[test]
fn owned_iterator_preserves_remaining_buffer() {
    let rb = Rb::<Array<u8, 3>>::from([1, 2, 3]);
    let mut iter = rb.into_iter();
    assert_eq!(iter.size_hint(), (3, None));
    assert_eq!(iter.next(), Some(1));
    let mut rb = iter.into_inner();
    assert_eq!(rb.first(), Some(&2));
    assert_eq!(rb.clear(), 2);
}

#[test]
fn exclusive_access_after_forgotten_borrowed_endpoints() {
    let mut rb = Rb::<Array<u8, 2>>::default();
    let (mut producer, consumer) = rb.split_ref();
    producer.try_push(7).unwrap();
    core::mem::forget(producer);
    core::mem::forget(consumer);
    // The stale held flags do not invalidate exclusive access to the owner.
    assert_eq!(rb.as_exclusive().try_pop(), Some(7));
    assert_eq!(rb.try_push(8), Ok(()));
    assert_eq!(rb.first(), Some(&8));
}

#[test]
fn send_and_sync_follow_access_capabilities() {
    fn send<T: Send>() {}
    fn sync<T: Sync>() {}
    type Buffer = StaticRb<Cell<u8>, 2>;
    send::<Cons<&'static Buffer>>();
    send::<Prod<&'static Buffer>>();
    sync::<Obs<&'static Buffer>>();
    sync::<Buffer>();
    sync::<Cons<&'static StaticRb<u8, 2>>>();
    sync::<Prod<&'static StaticRb<u8, 2>>>();
    send::<Exclusive<Buffer>>();
    send::<Exclusive<&'static mut Buffer>>();
    sync::<Exclusive<StaticRb<u8, 2>>>();
    sync::<Exclusive<&'static mut StaticRb<u8, 2>>>();
}

#[cfg(feature = "alloc")]
#[test]
fn retained_handles_only_observe_and_can_reacquire_endpoints() {
    use crate::{LocalRb, SharedRb, alias::Arc};
    use alloc::rc::Rc;
    let rb = Rc::new(LocalRb::<Array<u8, 2>>::default());
    let (mut producer, mut consumer) = rb.clone().split();
    producer.try_push(7).unwrap();
    assert_eq!(rb.occupied_len(), 1);
    assert_eq!(consumer.try_pop(), Some(7));
    drop(consumer);
    let mut consumer = Cons::new(rb.clone());
    producer.try_push(8).unwrap();
    assert_eq!(consumer.first(), Some(&8));
    assert_eq!(consumer.try_pop(), Some(8));

    let rb = Arc::new(SharedRb::<Array<u8, 2>>::default());
    let (mut producer, mut consumer) = rb.clone().split();
    producer.try_push(9).unwrap();
    assert_eq!(rb.occupied_len(), 1);
    assert_eq!(consumer.first(), Some(&9));
    assert_eq!(consumer.try_pop(), Some(9));
}

#[test]
fn exclusive_owns_or_borrows_and_recovers_the_buffer() {
    fn exercise<R: RbMut>(mut exclusive: Exclusive<R>) -> R
    where
        R::Rb: RingBuffer<Item = u8>,
    {
        exclusive.push_slice(&[1, 2, 3]);
        assert_eq!(exclusive.rb_mut().push_overwrite(4), Some(1));
        assert!(exclusive.iter().copied().eq([2, 3, 4]));
        assert_eq!(exclusive.try_pop(), Some(2));
        exclusive.into_inner()
    }

    let mut rb = exercise(Exclusive::new(Rb::<Array<u8, 3>>::default()));
    assert!(rb.pop_iter().eq([3, 4]));
    let borrowed = exercise(rb.as_exclusive());
    assert!(borrowed.pop_iter().eq([3, 4]));
    assert!(rb.is_empty());
}

#[cfg(feature = "alloc")]
#[test]
fn exclusive_tracks_replaced_storage_and_owns_item_destruction() {
    use alloc::{boxed::Box, rc::Rc};
    struct CountDrop(Rc<Cell<usize>>);
    impl Drop for CountDrop {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let item = || Box::new(CountDrop(drops.clone()));
    let mut exclusive = Exclusive::new(Rb::<crate::storage::Heap<Box<CountDrop>>>::new(1));
    assert!(exclusive.try_push(item()).is_ok());
    let replacement = Rb::<crate::storage::Heap<Box<CountDrop>>>::new(3);
    let old = core::mem::replace(exclusive.rb_mut(), replacement);
    assert_eq!(exclusive.capacity().get(), 3);
    assert!(exclusive.is_empty());
    drop(old);
    assert_eq!(drops.get(), 1);
    for _ in 0..3 {
        assert!(exclusive.try_push(item()).is_ok());
    }
    drop(exclusive.rb_mut().push_overwrite(item()));
    assert_eq!(drops.get(), 2);
    let mut rb = exclusive.into_inner();
    drop(rb.try_pop());
    assert_eq!(drops.get(), 3);
    drop(Exclusive::new(rb));
    assert_eq!(drops.get(), 5);
}

#[test]
fn exclusive_reborrows_and_recovers_after_forgotten_endpoints() {
    let mut rb = Rb::<Array<u8, 2>>::default();
    let mut exclusive = rb.as_exclusive();
    let (mut producer, consumer) = exclusive.rb_mut().split_ref();
    producer.try_push(7).unwrap();
    core::mem::forget(producer);
    core::mem::forget(consumer);
    assert_eq!(exclusive.try_pop(), Some(7));
    exclusive.try_push(8).unwrap();
    assert!(exclusive.read_is_held());
    assert!(exclusive.write_is_held());
    let rb = exclusive.into_inner();
    assert_eq!(rb.try_pop(), Some(8));
}

#[test]
fn endpoints_accept_owned_and_mutably_borrowed_buffers() {
    macro_rules! exercise {
        ($producer:ty, $consumer:ty) => {{
            let mut producer = <$producer>::new(Rb::<Array<u8, 2>>::default());
            producer.try_push(7).unwrap();
            let mut rb = producer.into_rb_ref();
            assert!(!rb.write_is_held());
            {
                let mut consumer = <$consumer>::new(&mut rb);
                assert_eq!(consumer.try_pop(), Some(7));
                assert!(consumer.read_is_held());
                let rb = consumer.into_rb_ref();
                assert!(!rb.read_is_held());
                rb.push_slice(&[8, 9]);
            }
            assert!(rb.pop_iter().eq([8, 9]));
        }};
    }
    exercise!(Prod<_>, Cons<_>);
    exercise!(crate::CachingProd<_>, crate::CachingCons<_>);
}

#[test]
fn cloned_observers_keep_the_same_buffer() {
    let rb = Rb::<Array<u8, 2>>::default();
    let mut producer = Prod::new(&rb);
    let observer = producer.observe();
    let clone = observer.clone();
    assert!(core::ptr::eq(observer.rb(), clone.rb()));
    producer.try_push(7).unwrap();
    assert_eq!(clone.occupied_len(), 1);
}
