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
pub mod shared;
pub mod storage;
pub mod stored;
mod utils;

#[cfg(test)]
mod tests;

pub use consumer::Consumer;
pub use local::LocalRb;
pub use observer::Observer;
pub use producer::Producer;
pub use ring_buffer::{RingBuffer, Split};
pub use shared::SharedRb;

pub mod prelude {
    #[cfg(feature = "alloc")]
    pub use super::stored::HeapRb;
    pub use super::{stored::StaticRb, Consumer, Observer, Producer, RingBuffer, Split};
}
