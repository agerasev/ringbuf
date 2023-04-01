#![no_std]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod local;
pub mod observer;
pub mod producer;
pub mod raw;
pub mod storage;
mod utils;
