#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod consumer;
pub mod producer;
pub mod ring_buffer;

mod transfer;

pub use consumer::AsyncConsumer;
pub use producer::AsyncProducer;
#[cfg(feature = "alloc")]
pub use ring_buffer::AsyncHeapRb;
pub use ring_buffer::AsyncRb;
pub use transfer::async_transfer;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests;
