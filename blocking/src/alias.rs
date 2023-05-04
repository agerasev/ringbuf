use crate::index::BlockingIndex;
use ringbuf::{index::SharedIndex, storage::Heap, Rb};

pub type BlockingRb<T> = Rb<Heap<T>, BlockingIndex<SharedIndex>, BlockingIndex<SharedIndex>>;
