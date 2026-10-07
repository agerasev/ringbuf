use core::{num::NonZeroUsize, ops::Range};

pub fn assert_capacity(capacity: usize) {
    assert!(capacity > 0, "capacity must be non-zero");
    assert!(capacity <= usize::MAX / 2, "capacity exceeds usize::MAX / 2");
}

/// Returns a pair of ranges between `start` and `end` indices in a ring buffer with specific `capacity`.
///
/// `start` and `end` may be arbitrarily large, but their mathematical forward distance
/// `(end - start) mod (2 * capacity)` must be at most `capacity`.
/// Actual indices are taken modulo `capacity`.
///
/// The first range starts from `start`. If the first slice is empty then second slice is empty too.
pub fn ranges(capacity: NonZeroUsize, start: usize, end: usize) -> (Range<usize>, Range<usize>) {
    let (head_quo, head_rem) = (start / capacity, start % capacity);
    let (tail_quo, tail_rem) = (end / capacity, end % capacity);

    if head_quo % 2 == tail_quo % 2 {
        (head_rem..tail_rem, 0..0)
    } else {
        (head_rem..capacity.get(), 0..tail_rem)
    }
}

#[cfg(test)]
mod tests {
    use super::ranges;
    use core::num::NonZeroUsize;

    #[test]
    fn large_indices() {
        let capacity = NonZeroUsize::new(1).unwrap();
        assert_eq!(ranges(capacity, usize::MAX - 1, usize::MAX), (0..1, 0..0));
        assert_eq!(ranges(capacity, usize::MAX, 0), (0..1, 0..0));
        assert_eq!(ranges(capacity, usize::MAX, usize::MAX), (0..0, 0..0));
    }
}
