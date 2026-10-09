//! Capacity, allocation, and immediate bulk-operation errors.
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError {
    pub capacity: usize,
}
impl CapacityError {
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
            Self::Capacity(e) => e.fmt(f),
            Self::Allocation(e) => e.fmt(f),
        }
    }
}
#[cfg(feature = "alloc")]
impl core::error::Error for CreateError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(match self {
            Self::Capacity(e) => e,
            Self::Allocation(e) => e,
        })
    }
}

/// Exact operations never make partial progress when returning this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactError {
    TooLarge { requested: usize, capacity: usize },
    Unavailable { requested: usize, available: usize },
}
impl ExactError {
    pub(crate) fn check(requested: usize, capacity: usize, available: usize) -> Result<(), Self> {
        if requested > capacity {
            Err(Self::TooLarge { requested, capacity })
        } else if requested > available {
            Err(Self::Unavailable { requested, available })
        } else {
            Ok(())
        }
    }
}
impl fmt::Display for ExactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl core::error::Error for ExactError {}
