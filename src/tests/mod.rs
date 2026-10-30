#[cfg(feature = "test_local")]
use crate::LocalRb as Rb;
#[cfg(not(feature = "test_local"))]
use crate::SharedRb as Rb;

mod access;
mod basic;
mod caching;
mod capacity;
#[cfg(feature = "alloc")]
mod drop;
mod fmt_write;
mod hold;
mod init;
mod iter;
#[cfg(feature = "alloc")]
mod new;
mod overwrite;
#[cfg(feature = "std")]
mod read_write;
#[cfg(feature = "std")]
mod shared;
#[cfg(feature = "alloc")]
mod skip;
mod slice;
mod unsized_;
#[cfg(feature = "alloc")]
mod zero_sized;

mod raw;

mod ownership;
