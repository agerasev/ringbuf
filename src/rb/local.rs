/// Ring buffer using compact single-threaded indices and ownership tracking.
pub type LocalRb<S> = super::Rb<S, crate::indices::LocalIndices, crate::markers::LocalMarkers>;
