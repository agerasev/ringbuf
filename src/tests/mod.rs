use crate::SharedRb as Rb;

mod access;
mod basic;
#[cfg(feature = "alloc")]
mod drop;
mod fmt_write;
mod frozen;
mod iter;
mod overwrite;
#[cfg(feature = "std")]
mod read_write;
#[cfg(feature = "std")]
mod shared;
#[cfg(feature = "alloc")]
mod skip;
mod slice;
