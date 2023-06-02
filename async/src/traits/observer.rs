use ringbuf::traits::Observer;

pub trait AsyncObserver: Observer {
    fn is_closed(&self) -> bool;
    fn close(&self);
}
