use std::{
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct TimeoutIterator {
    start: Instant,
    timeout: Option<Duration>,
}

impl TimeoutIterator {
    pub fn new(timeout: Option<Duration>) -> Self {
        Self {
            start: Instant::now(),
            timeout,
        }
    }
}

impl Iterator for TimeoutIterator {
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

#[derive(Default)]
pub struct Semaphore {
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Semaphore {
    pub fn wait<F: Fn() -> bool>(&self, f: F, timeout: Option<Duration>) -> bool {
        let mut guard_slot = Some(self.mutex.lock().unwrap());
        for timeout in TimeoutIterator::new(timeout) {
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
    pub fn notify<F: FnOnce()>(&self, f: F) {
        let _guard = self.mutex.lock();
        f();
        self.condvar.notify_one();
    }
}
