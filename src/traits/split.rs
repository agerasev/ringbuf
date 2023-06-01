use crate::traits::{Consumer, Producer, RingBuffer};

pub trait Split: RingBuffer {
    type Prod: Producer;
    type Cons: Consumer;

    fn split(self) -> (Self::Prod, Self::Cons);
}

pub trait SplitRef<'a>: RingBuffer + 'a {
    type RefProd: Producer + 'a;
    type RefCons: Consumer + 'a;

    fn split_ref(&'a mut self) -> (Self::RefProd, Self::RefCons);
}
