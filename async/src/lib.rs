#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod consumer;
pub mod producer;
pub mod ring_buffer;

#[cfg(feature = "alloc")]
mod alias;
mod transfer;

#[cfg(feature = "alloc")]
pub use alias::{AsyncHeapConsumer, AsyncHeapProducer, AsyncHeapRb};
pub use consumer::AsyncConsumer;
pub use producer::AsyncProducer;
pub use ring_buffer::AsyncRb;
pub use transfer::async_transfer;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests;
