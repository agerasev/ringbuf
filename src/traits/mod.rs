/// Consumer functionality.
pub mod consumer;
/// Observer functionality.
pub mod observer;
/// Producer functionality.
pub mod producer;
/// Owning ring buffer functionality.
pub mod ring_buffer;
mod split;
pub(crate) mod utils;

pub use consumer::Consumer;
pub use observer::Observer;
pub use producer::Producer;
pub use ring_buffer::RingBuffer;
pub use split::{Split, SplitRef};
pub use utils::Delegate;

pub mod raw;
pub use raw::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};

pub use utils::Delegate as Based;

/// Optional endpoint presence information. Absence always means closed.
pub trait Presence: Observer {
    fn read_is_held(&self) -> bool;
    fn write_is_held(&self) -> bool;
}
