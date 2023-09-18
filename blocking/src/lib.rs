#![no_std]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod rb;
pub mod sync;
pub mod wrap;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use ringbuf::traits;

pub use alias::*;
pub use rb::BlockingRb;
pub use wrap::{BlockingCons, BlockingProd, WaitError};
