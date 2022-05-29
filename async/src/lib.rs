#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod consumer;
mod producer;
mod ring_buffer;
mod transfer;

pub use consumer::*;
pub use producer::*;
pub use ring_buffer::*;
pub use transfer::*;

#[cfg(test)]
mod tests;
