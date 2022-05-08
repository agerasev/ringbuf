use super::GlobalConsumer;
use crate::ring_buffer::{Container, RingBufferRef};
use futures::task::AtomicWaker;

struct AsyncConsumer<T, C: Container<T>, R: RingBufferRef<T, C>> {
    basic: GlobalConsumer<T, C, R>,
    waker: AtomicWaker,
}
