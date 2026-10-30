use core::fmt;

/// An asynchronous wait could not satisfy its requested threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaitError {
    /// The opposite endpoint is closed.
    Closed,
    /// The requested threshold exceeds the buffer's capacity.
    TooLarge { requested: usize, capacity: usize },
}
impl fmt::Display for WaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("opposite endpoint is closed"),
            Self::TooLarge { requested, capacity } => write!(f, "requested {requested} slots exceeds capacity {capacity}"),
        }
    }
}
impl core::error::Error for WaitError {}
