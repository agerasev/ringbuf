//#[cfg(feature = "alloc")]
//use alloc::sync::Arc;

/// Splits the ring buffer into producer and consumer.
pub trait Split {
    type Producer;
    type Consumer;
    fn split(self) -> (Self::Producer, Self::Consumer);
}

/*
/// Splits ring buffer into producer and consumer.
///
/// This method consumes the ring buffer and puts it on heap in [`Arc`]. If you don't want to use heap the see [`Self::split_static`].
#[cfg(feature = "alloc")]
pub fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>) {
    let arc = Arc::new(self);
    unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
}

/// Splits ring buffer into producer and consumer without using the heap.
///
/// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
pub fn split_static(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>) {
    unsafe { (Producer::new(self), Consumer::new(self)) }
}
*/
