use core::time::Duration;
#[cfg(feature = "std")]
use std::sync::{Condvar, Mutex};

pub trait Instant {
    fn now() -> Self;
    fn elapsed(&self) -> Duration;
}

pub trait Semaphore {
    type Instant: Instant;

    fn new() -> Self;
    fn wait<F: Fn() -> bool>(&self, f: F, timeout: Option<Duration>) -> bool;
    fn notify<F: FnOnce()>(&self, f: F);
}

#[cfg(feature = "std")]
pub use std::time::Instant as StdInstant;

impl Instant for StdInstant {
    fn now() -> Self {
        StdInstant::now()
    }
    fn elapsed(&self) -> Duration {
        StdInstant::elapsed(self)
    }
}

#[cfg(feature = "std")]
#[derive(Default)]
pub struct StdSemaphore {
    condvar: Condvar,
    mutex: Mutex<()>,
}

#[cfg(feature = "std")]
impl Semaphore for StdSemaphore {
    type Instant = StdInstant;

    fn new() -> Self {
        Self {
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }
    fn wait<F: Fn() -> bool>(&self, f: F, timeout: Option<Duration>) -> bool {
        if f() {
            return true;
        }
        let mut guard_slot = Some(self.mutex.lock().unwrap());
        for timeout in TimeoutIterator::<Self::Instant>::new(timeout) {
            if f() {
                return true;
            }
            let guard = guard_slot.take().unwrap();
            guard_slot.replace(match timeout {
                Some(t) => self.condvar.wait_timeout(guard, t).unwrap().0,
                None => self.condvar.wait(guard).unwrap(),
            });
        }
        f()
    }
    fn notify<F: FnOnce()>(&self, f: F) {
        let _guard = self.mutex.lock();
        f();
        self.condvar.notify_all();
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TimeoutIterator<I: Instant> {
    start: I,
    timeout: Option<Duration>,
}

impl<I: Instant> TimeoutIterator<I> {
    pub fn new(timeout: Option<Duration>) -> Self {
        Self { start: I::now(), timeout }
    }
}

impl<I: Instant> Iterator for TimeoutIterator<I> {
    type Item = Option<Duration>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.timeout {
            Some(dur) => {
                let elapsed = self.start.elapsed();
                if dur > elapsed {
                    Some(Some(dur - elapsed))
                } else {
                    None
                }
            }
            None => Some(None),
        }
    }
}
