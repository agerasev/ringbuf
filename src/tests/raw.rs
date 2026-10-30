use crate::{
    StaticRb,
    traits::{consumer::DelegateConsumer, observer::DelegateObserver, producer::DelegateProducer, ring_buffer::DelegateRingBuffer, *},
    wrap::{Cons, Prod},
};

struct Wrapper<B>(B);
impl<B> Based for Wrapper<B> {
    type Base = B;
    fn base(&self) -> &B {
        &self.0
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.0
    }
}
impl<B: Raw> DelegateObserver for Wrapper<B> {}
// SAFETY: the wrapper owns its base and forwards all methods to that same base.
unsafe impl<B: Raw> DelegateRaw for Wrapper<B> {}
impl<B: Consumer> DelegateConsumer for Wrapper<B> {}
impl<B: Producer> DelegateProducer for Wrapper<B> {}
impl<B: RingBuffer> DelegateRingBuffer for Wrapper<B> {}

impl<B> AsRef<Self> for Wrapper<B> {
    fn as_ref(&self) -> &Self {
        panic!("a borrowed handle must not rely on a user-defined AsRef")
    }
}

#[test]
fn custom_delegation_preserves_endpoint_access() {
    let mut rb = StaticRb::<u8, 2>::default();
    let (producer, consumer) = rb.split_ref();
    let mut producer = Wrapper(producer);
    let mut consumer = Wrapper(consumer);
    producer.try_push(7).unwrap();
    assert_eq!(consumer.first(), Some(&7));
    assert_eq!(consumer.try_pop(), Some(7));
    assert!(consumer.is_empty());
}

#[test]
fn borrowed_handle_uses_referenced_buffer() {
    let rb = Wrapper(StaticRb::<u8, 2>::default());
    let mut producer = Prod::new(&rb);
    let mut consumer = Cons::new(&rb);
    producer.try_push(7).unwrap();
    assert_eq!(rb.occupied_len(), 1);
    assert_eq!(consumer.try_pop(), Some(7));
}
