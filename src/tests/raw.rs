use crate::{
    StaticRb,
    traits::{consumer::DelegateConsumer, observer::DelegateObserver, producer::DelegateProducer, *},
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
