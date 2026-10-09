/// Ring buffer using atomic indices and ownership tracking.
pub type SharedRb<S> = super::Rb<S, crate::indices::AtomicIndices, crate::markers::AtomicMarkers>;
