#![no_std]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod consumer;
mod init;
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
pub use ring_buffer::{RingBuffer, Split};

pub mod prelude {
    pub use super::{
        consumer::Consumer,
        observer::Observer,
        producer::Producer,
        ring_buffer::{RingBuffer, Split},
    };
}
