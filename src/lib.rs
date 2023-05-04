#![no_std]
#![allow(clippy::type_complexity)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
//mod cached;
pub mod consumer;
pub mod index;
mod observer;
pub mod producer;
mod rb;
pub mod ref_;
mod ring_buffer;
pub mod storage;
mod transfer;
mod utils;

#[cfg(test)]
mod tests;

pub use alias::*;
//pub use cached::{CachedCons, CachedProd};
pub use rb::{Cons, Prod, Rb};
pub use transfer::transfer;

pub mod traits {
    pub use crate::{
        consumer::Consumer,
        observer::Observer,
        producer::Producer,
        ring_buffer::{RingBuffer, Split},
    };
}

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
