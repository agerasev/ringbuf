#![no_std]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod cached;
pub mod consumer;
mod local;
mod observer;
pub mod producer;
pub mod raw;
mod ring_buffer;
mod shared;
pub mod storage;
mod transfer;
mod utils;

#[cfg(test)]
mod tests;

pub use cached::{CachedCons, CachedProd};
pub use consumer::Cons;
pub use local::LocalRb;
pub use producer::Prod;
pub use shared::SharedRb;
pub use transfer::transfer;

pub mod traits {
    pub use crate::{
        consumer::Consumer, observer::Observer, producer::Producer, ring_buffer::RingBuffer,
    };
}
