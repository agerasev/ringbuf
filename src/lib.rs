#![no_std]
#![allow(clippy::type_complexity)]
#![allow(clippy::missing_safety_doc)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod halves;
pub mod rb;
pub mod storage;
pub mod traits;
mod transfer;
mod utils;

#[cfg(test)]
mod tests;

pub use alias::*;
pub use halves::{CachedCons, CachedProd, Cons, Obs, Prod};
pub use rb::{LocalRb, SharedRb};
pub use traits::{consumer, producer};
pub use transfer::transfer;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
