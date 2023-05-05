#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod consumer;
pub mod index;
pub mod producer;
mod transfer;

pub use alias::*;
pub use consumer::AsyncConsumer;
pub use producer::AsyncProducer;
pub use transfer::async_transfer;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests;
