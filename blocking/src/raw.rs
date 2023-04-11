pub use ringbuf::raw::RawRb;
use std::time::Duration;

pub trait RawBlockingRb: RawRb {
    fn wait_write<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool;
    fn wait_read<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool;
}
