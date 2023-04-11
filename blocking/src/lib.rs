pub mod raw;
mod rb;
mod utils;

#[cfg(test)]
mod tests;

pub use rb::{PopAllIter, Rb};

pub mod traits {
    pub use crate::rb::{
        Consumer as BlockingConsumer, Producer as BlockingProducer,
        RingBuffer as BlockingRingBuffer,
    };
}
