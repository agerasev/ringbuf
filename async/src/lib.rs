#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub use ringbuf;

mod consumer;
mod producer;
mod ring_buffer;
mod transfer;

pub mod counter;

pub use consumer::*;
pub use producer::*;
pub use ring_buffer::*;
pub use transfer::*;

#[cfg(test)]
mod tests;
