use super::Rb;
#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{storage::Ref, traits::*};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
    slice,
};

const MAX_CAPACITY: usize = usize::MAX / 2;
const MODULUS: usize = 2 * MAX_CAPACITY;
const STARTS: [usize; 5] = [0, MAX_CAPACITY - 1, MAX_CAPACITY, MODULUS - 2, MODULUS - 1];

fn storage(capacity: usize) -> Ref<'static, ()> {
    // Zero-sized elements need no allocation; the pointer is non-null and aligned.
    unsafe { slice::from_raw_parts_mut(NonNull::<MaybeUninit<()>>::dangling().as_ptr(), capacity) }.into()
}

fn empty_at(index: usize) -> ManuallyDrop<Rb<Ref<'static, ()>>> {
    // Equal indices describe an empty buffer. Suppress Drop so a regression cannot
    // make unwinding try to drain an enormous buffer. The storage owns no allocation.
    ManuallyDrop::new(unsafe { Rb::from_raw_parts(storage(MAX_CAPACITY), index, index) })
}

#[test]
fn maximum_capacity_direct() {
    for start in STARTS {
        let mut rb = empty_at(start);
        assert_eq!(rb.occupied_len(), 0);
        assert_eq!(rb.vacant_len(), MAX_CAPACITY);
        let (left, right) = rb.vacant_slices();
        assert_eq!(left.len() + right.len(), MAX_CAPACITY);

        assert_eq!(rb.push_slice(&[(); 3]), 3);
        assert_eq!(rb.occupied_len(), 3);
        assert_eq!(rb.vacant_len(), MAX_CAPACITY - 3);
        assert_eq!(rb.pop_slice(&mut [(); 2]), 2);
        assert_eq!(rb.pop_iter().count(), 1);
        assert!(rb.is_empty());

        assert_eq!(rb.push_iter([(); 3].into_iter()), 3);
        assert_eq!(rb.skip(2), 2);
        assert_eq!(rb.clear(), 1);
        assert_eq!(rb.vacant_len(), MAX_CAPACITY);
    }
}

#[test]
fn maximum_capacity_cached() {
    for start in STARTS {
        let mut rb = empty_at(start);
        let (mut prod, mut cons) = rb.split_ref();
        assert_eq!(prod.try_push(()), Ok(()));
        assert_eq!(prod.try_push(()), Ok(()));
        assert_eq!(cons.occupied_len(), 2);
        assert_eq!(prod.vacant_len(), MAX_CAPACITY - 2);
        assert_eq!(cons.try_pop(), Some(()));
        assert_eq!(cons.try_pop(), Some(()));
        assert_eq!(cons.try_pop(), None);

        assert_eq!(prod.push_slice(&[(); 3]), 3);
        assert_eq!(cons.pop_slice(&mut [(); 2]), 2);
        assert_eq!(cons.pop_iter().count(), 1);
        assert_eq!(prod.vacant_len(), MAX_CAPACITY);
    }
}

#[test]
fn maximum_capacity_bulk_advance() {
    for start in STARTS {
        let mut rb = empty_at(start);
        let expected = ((start as u128 + MAX_CAPACITY as u128) % MODULUS as u128) as usize;
        // All vacant slots contain (): there are no bytes to initialize.
        unsafe { rb.advance_write_index(MAX_CAPACITY) };
        assert_eq!(rb.write_index(), expected);
        assert_eq!(rb.occupied_len(), MAX_CAPACITY);
        assert_eq!(rb.vacant_len(), 0);
        assert!(rb.is_full());
        assert_eq!(rb.try_push(()), Err(()));
        let (left, right) = rb.occupied_slices();
        assert_eq!(left.len() + right.len(), MAX_CAPACITY);
        let (left, right) = rb.vacant_slices();
        assert_eq!(left.len() + right.len(), 0);

        // Consume all units at once; they have no destructor or bytes to move.
        unsafe { rb.advance_read_index(MAX_CAPACITY) };
        assert_eq!(rb.read_index(), expected);
        assert_eq!(rb.occupied_len(), 0);
        assert_eq!(rb.vacant_len(), MAX_CAPACITY);
        assert!(rb.is_empty());
    }
}

#[test]
#[should_panic(expected = "capacity must be non-zero")]
fn zero_capacity() {
    let _rb = ManuallyDrop::new(unsafe { Rb::from_raw_parts(storage(0), 0, 0) });
}

#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn capacity_with_zero_modulus() {
    let _rb = ManuallyDrop::new(unsafe { Rb::from_raw_parts(storage(MAX_CAPACITY + 1), 0, 0) });
}

#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn capacity_with_wrapped_modulus() {
    let _rb = ManuallyDrop::new(unsafe { Rb::from_raw_parts(storage(MAX_CAPACITY + 2), 0, 0) });
}

#[cfg(feature = "alloc")]
#[test]
fn maximum_heap_capacity() {
    for mut rb in [Rb::<Heap<()>>::new(MAX_CAPACITY), Rb::<Heap<()>>::try_new(MAX_CAPACITY).unwrap()] {
        assert_eq!(rb.capacity().get(), MAX_CAPACITY);
        assert_eq!(rb.try_push(()), Ok(()));
        assert_eq!(rb.try_push(()), Ok(()));
        assert_eq!(rb.occupied_len(), 2);
        assert_eq!(rb.vacant_len(), MAX_CAPACITY - 2);
        assert_eq!(rb.pop_iter().count(), 2);
    }
}

#[cfg(feature = "alloc")]
#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn excessive_heap_capacity() {
    let _rb = ManuallyDrop::new(Rb::<Heap<()>>::new(MAX_CAPACITY + 1));
}

#[cfg(feature = "alloc")]
#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn excessive_heap_capacity_try_new() {
    let _rb = ManuallyDrop::new(Rb::<Heap<()>>::try_new(usize::MAX).unwrap());
}

#[cfg(feature = "alloc")]
#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn excessive_vec_capacity() {
    let _rb = ManuallyDrop::new(Rb::from(alloc::vec![(); MAX_CAPACITY + 2]));
}

#[cfg(feature = "alloc")]
#[test]
#[should_panic(expected = "capacity exceeds usize::MAX / 2")]
fn excessive_boxed_slice_capacity() {
    let _rb = ManuallyDrop::new(Rb::from(alloc::vec![(); MAX_CAPACITY + 2].into_boxed_slice()));
}
