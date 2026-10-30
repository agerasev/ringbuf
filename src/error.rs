//! Errors returned by fallible buffer constructors.
use core::fmt;

/// A buffer capacity outside `1..=usize::MAX / 2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError {
    pub capacity: usize,
}
impl CapacityError {
    /// Validate a buffer capacity, including for zero-sized items.
    pub fn check(capacity: usize) -> Result<(), Self> {
        if capacity == 0 || capacity > usize::MAX / 2 {
            Err(Self { capacity })
        } else {
            Ok(())
        }
    }
}
impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "capacity {} is outside 1..=usize::MAX / 2", self.capacity)
    }
}
impl core::error::Error for CapacityError {}

/// A heap buffer could not be created.
#[cfg(feature = "alloc")]
#[derive(Debug)]
pub enum CreateError {
    Capacity(CapacityError),
    Allocation(alloc::collections::TryReserveError),
}
#[cfg(feature = "alloc")]
impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity(error) => error.fmt(f),
            Self::Allocation(error) => error.fmt(f),
        }
    }
}
#[cfg(feature = "alloc")]
impl core::error::Error for CreateError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(match self {
            Self::Capacity(error) => error,
            Self::Allocation(error) => error,
        })
    }
}
