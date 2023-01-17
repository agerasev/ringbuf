use ringbuf::{HeapRb, Rb};

fn main() {
    let mut rb = HeapRb::<i32>::new(2);

    assert_eq!(rb.push_overwrite(0), None);
    assert_eq!(rb.push_overwrite(1), None);
    assert_eq!(rb.push_overwrite(2), Some(0));

    assert_eq!(rb.pop(), Some(1));
    assert_eq!(rb.pop(), Some(2));
    assert_eq!(rb.pop(), None);
}
