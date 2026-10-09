use ringbuf::{ArrayRb, ExactError, traits::*};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn exact_operations_are_atomic_across_wraparound() {
    let mut rb = ArrayRb::<i32, 3>::default();
    for _ in 0..10 {
        assert_eq!(rb.try_push_array([1, 2]), Ok(()));
        assert!(matches!(rb.try_push_slice(&[3, 4]), Err(ExactError::Unavailable { .. })));
        let mut output = [99; 3];
        assert!(rb.try_pop_slice(&mut output).is_err());
        assert_eq!(output, [99; 3]);
        assert_eq!(rb.try_pop_array::<2>(), Ok([1, 2]));
        assert!(matches!(rb.try_pop_array::<4>(), Err(ExactError::TooLarge { .. })));
        assert!(rb.is_empty());
        assert_eq!(rb.try_push_array([]), Ok(()));
        assert_eq!(rb.try_pop_array::<0>(), Ok([]));
    }
}

#[test]
fn arrays_move_noncopy_values_and_failure_returns_ownership() {
    let mut rb = ArrayRb::<String, 2>::default();
    rb.try_push_array(["a".into(), "b".into()]).unwrap();
    let (_, values) = rb.try_push_array([String::from("c")]).unwrap_err();
    assert_eq!(values, ["c"]);
    assert_eq!(rb.try_pop_array::<2>().unwrap(), ["a", "b"]);
}

#[test]
fn initialization_panic_preserves_the_initialized_prefix() {
    let mut rb = ArrayRb::<String, 3>::default();
    let (mut p, mut c) = rb.split_ref();
    let mut d = p.defer();
    let mut n = 0;
    assert!(
        catch_unwind(AssertUnwindSafe(|| d.fill_with(3, || {
            n += 1;
            if n == 3 {
                panic!("initializer");
            }
            n.to_string()
        })))
        .is_err()
    );
    assert_eq!(d.pending_len(), 2);
    assert!(c.try_pop().is_none());
    drop(d);
    assert_eq!(c.try_pop_array::<2>().unwrap(), ["1", "2"]);
}

#[cfg(feature = "alloc")]
#[test]
fn construction_returns_capacity_and_allocation_errors() {
    use ringbuf::{CreateError, HeapRb};
    assert!(matches!(HeapRb::<u8>::try_new(0), Err(CreateError::Capacity(_))));
    assert!(matches!(HeapRb::<u8>::try_new(usize::MAX), Err(CreateError::Capacity(_))));
    assert!(matches!(HeapRb::<u64>::try_new(usize::MAX / 2), Err(CreateError::Allocation(_))));
    let rb = HeapRb::<()>::try_new(usize::MAX / 2).unwrap();
    assert_eq!(rb.capacity().get(), usize::MAX / 2);
}
