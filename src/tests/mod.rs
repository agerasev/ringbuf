#[cfg(feature = "alloc")]
mod access;
#[cfg(feature = "alloc")]
mod base;
#[cfg(feature = "alloc")]
mod drop;
#[cfg(feature = "alloc")]
mod iter;
#[cfg(feature = "alloc")]
mod skip;
#[cfg(feature = "alloc")]
mod slice;

#[cfg(feature = "std")]
mod message;
#[cfg(feature = "std")]
mod read_write;

#[cfg(feature = "bench")]
mod bench;
