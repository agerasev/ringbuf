use crate::consumer::AsyncConsumer;
use crate::producer::AsyncProducer;
use ringbuf::traits::RingBuffer;

pub trait AsyncRingBuffer: RingBuffer + AsyncProducer + AsyncConsumer {
    fn wake_writer(&self);
    fn wake_reader(&self);
}
