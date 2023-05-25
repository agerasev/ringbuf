#![no_std]
#![allow(clippy::type_complexity)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod rb;
pub mod traits;
mod transfer;
mod utils;

#[cfg(test)]
mod tests;

pub use rb::*;
pub use traits::consumer;
pub use traits::producer;

pub use consumer::Cons;
pub use producer::Prod;
pub use transfer::transfer;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
