#![no_std]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod consumer;
pub mod local;
pub mod observer;
pub mod producer;
pub mod raw;
pub mod ring_buffer;
pub mod storage;
mod utils;

pub use consumer::Consumer;
pub use observer::Observer;
pub use producer::Producer;
pub use ring_buffer::RingBuffer;

pub mod prelude {
    pub use super::{Consumer, Observer, Producer, RingBuffer};
}
