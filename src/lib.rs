use std::sync::{Arc, atomic::{AtomicUsize}};
use std::io::{self, Read, Write};


const ATOMIC_TRIES: usize = 16;

pub struct Rb {
    data: Vec<u8>,
    alloc_head: AtomicUsize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

pub struct RbProducer {
    rb: Arc<Rb>,
}

pub struct RbConsumer {
    rb: Arc<Rb>,
}

impl Rb {
    pub fn new(capacity: usize) -> Rb {
        Rb {
            data: vec!(0; capacity),
            alloc_head: AtomicUsize::new(0),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn split(self) -> (RbProducer, RbConsumer) {
        let arc = Arc::new(self);
        (
            RbProducer { rb: arc.clone() },
            RbConsumer { rb: arc },
        )
    }
}

impl Clone for RbProducer {
    fn clone(&self) -> Self {
        RbProducer { rb: self.rb.clone() }
    }
}

impl Write for RbProducer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for RbConsumer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
