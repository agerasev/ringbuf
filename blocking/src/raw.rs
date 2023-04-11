use std::time::Duration;

mod base {
    pub use ringbuf::raw::{RawCons, RawProd};
}

pub trait RawProd: base::RawProd {
    fn wait_write<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool;
}
pub trait RawCons: base::RawCons {
    fn wait_read<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool;
}
pub trait RawRb: RawProd + RawCons {}
