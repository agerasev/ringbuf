pub mod consumer;
pub mod observer;
pub mod producer;

pub use consumer::AsyncConsumer;
pub use observer::AsyncObserver;
pub use producer::AsyncProducer;

pub trait AsyncRingBuffer: ringbuf::traits::RingBuffer + AsyncProducer + AsyncConsumer {}
