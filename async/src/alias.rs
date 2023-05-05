use crate::index::AsyncIndex;
use ringbuf::{index::SharedIndex, storage::Heap, Rb};

pub type AsyncRb<T> = Rb<Heap<T>, AsyncIndex<SharedIndex>, AsyncIndex<SharedIndex>>;
