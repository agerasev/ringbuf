use ringbuf::{ArrayRb, traits::*};

fn main() {
    let mut rb = ArrayRb::<i32, 4>::default();
    let (mut producer, mut consumer) = rb.split_ref();
    {
        let mut batch = producer.defer();
        batch.try_push_array([10, 20]).unwrap();
        assert_eq!(consumer.try_pop(), None);
        assert_eq!(batch.undo_push(), Some(20));
        batch.commit();
        assert_eq!(consumer.try_pop(), Some(10));
        batch.fetch();
        batch.try_push(30).unwrap();
    }
    let mut batch = consumer.into_deferred();
    assert_eq!(batch.try_pop(), Some(30));
    batch.undo_pop(31).unwrap();
    assert_eq!(batch.try_pop(), Some(31));
    batch.commit();
}
