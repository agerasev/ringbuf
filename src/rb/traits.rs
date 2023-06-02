use crate::traits::{Consumer, Producer, RingBuffer, Split, SplitRef};
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

pub unsafe trait RbRef: Clone {
    type Target: RingBuffer;
    fn deref(&self) -> &Self::Target;
}

unsafe impl<'a, B: RingBuffer> RbRef for &'a B {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Rc<B> {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Arc<B> {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}

pub unsafe trait AsRb {
    type Rb: RingBuffer;
    fn as_rb(&self) -> &Self::Rb;
}

pub unsafe trait GenSplit<B: RingBuffer = Self>: RingBuffer {
    type GenProd: Producer + AsRb<Rb = B>;
    type GenCons: Consumer + AsRb<Rb = B>;

    fn gen_split(this: B) -> (Self::GenProd, Self::GenCons);
}

pub unsafe trait GenSplitRef<'a, B: RingBuffer + 'a = Self>: RingBuffer + 'a {
    type GenRefProd: Producer + AsRb<Rb = B> + 'a;
    type GenRefCons: Consumer + AsRb<Rb = B> + 'a;

    fn gen_split_ref(this: &'a mut B) -> (Self::GenRefProd, Self::GenRefCons);
}

impl<B: GenSplit> Split for B {
    type Prod = B::GenProd;
    type Cons = B::GenCons;

    fn split(self) -> (Self::Prod, Self::Cons) {
        B::gen_split(self)
    }
}
impl<'a, B: GenSplitRef<'a> + 'a> SplitRef<'a> for B {
    type RefProd = B::GenRefProd;
    type RefCons = B::GenRefCons;

    fn split_ref(&'a mut self) -> (Self::RefProd, Self::RefCons) {
        B::gen_split_ref(self)
    }
}
