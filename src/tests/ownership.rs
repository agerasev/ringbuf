use super::Rb;
use crate::{
    StaticRb,
    storage::Array,
    traits::*,
    wrap::{Cons, Obs, Prod},
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
