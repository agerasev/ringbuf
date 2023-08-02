pub trait Delegate {
    type Base;
    fn base(&self) -> &Self::Base;
}
pub trait DelegateMut: Delegate {
    fn base_mut(&mut self) -> &mut Self::Base;
}

pub use super::consumer::DelegateConsumer as Consumer;
pub use super::observer::DelegateObserver as Observer;
pub use super::producer::DelegateProducer as Producer;
pub use super::ring_buffer::DelegateRingBuffer as RingBuffer;
