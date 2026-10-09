#![no_std]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod endpoint;
pub mod rb;
pub mod sync;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use ringbuf::traits;

pub use alias::*;
pub use endpoint::{BlockingCons, BlockingProd, WaitError};
pub use rb::BlockingRb;

pub use endpoint as wrap;
pub use rb::BlockingMarkers;
