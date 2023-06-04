#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
mod alias;
pub mod halves;
pub mod rb;
pub mod sync;
pub mod traits;

#[cfg(all(test, feature = "std"))]
mod tests;

#[cfg(feature = "alloc")]
pub use alias::*;
pub use halves::{BlockingCons, BlockingProd};
pub use rb::BlockingRb;
