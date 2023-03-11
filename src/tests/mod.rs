#[cfg(feature = "alloc")]
mod access;
#[cfg(feature = "alloc")]
mod basic;
#[cfg(feature = "alloc")]
mod cached;
#[cfg(feature = "alloc")]
mod drop;
#[cfg(feature = "alloc")]
mod iter;
#[cfg(feature = "alloc")]
mod overwrite;
#[cfg(feature = "alloc")]
mod skip;
#[cfg(feature = "alloc")]
mod slice;

#[cfg(feature = "std")]
mod message;
#[cfg(feature = "std")]
mod read_write;

mod fmt_write;
