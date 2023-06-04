use crate::traits::{Consumer, Producer, RingBuffer};

pub trait Split: RingBuffer {
    type Prod: Producer;
    type Cons: Consumer;

    fn split(self) -> (Self::Prod, Self::Cons);
}

pub trait SplitRef: RingBuffer {
    type RefProd<'a>: Producer + 'a
    where
        Self: 'a;
    type RefCons<'a>: Consumer + 'a
    where
        Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>);
}
