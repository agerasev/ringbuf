use super::{
    super::frozen::{FrozenCons, FrozenProd},
    CachedCons, CachedProd,
};
use crate::{
    delegate_consumer, delegate_frozen_consumer, delegate_frozen_producer, delegate_observer, delegate_producer,
    rbs::based::{Based, RbRef},
    traits::{Consumer, FrozenConsumer, FrozenProducer, Observer, Producer},
};

impl<R: RbRef> CachedProd<R> {
    #[inline]
    pub fn fetch(&self) {
        self.frozen.fetch()
    }

    pub fn freeze(&mut self) -> FrozenCachedProdRef<R> {
        self.frozen.fetch();
        FrozenCachedProdRef { frozen: &mut self.frozen }
    }
    pub fn into_frozen(self) -> FrozenCachedProd<R> {
        self.frozen.fetch();
        FrozenCachedProd { frozen: self.frozen }
    }
}
impl<R: RbRef> CachedCons<R> {
    #[inline]
    pub fn fetch(&self) {
        self.frozen.fetch()
    }

    pub fn freeze(&mut self) -> FrozenCachedConsRef<R> {
        self.frozen.fetch();
        FrozenCachedConsRef { frozen: &mut self.frozen }
    }
    pub fn into_frozen(self) -> FrozenCachedCons<R> {
        self.frozen.fetch();
        FrozenCachedCons { frozen: self.frozen }
    }
}

pub type FrozenCachedProd<R> = CachedProd<R, false>;
pub type FrozenCachedCons<R> = CachedCons<R, false>;

impl<R: RbRef> FrozenCachedProd<R> {
    fn frozen(&self) -> &FrozenProd<R> {
        &self.frozen
    }
    fn frozen_mut(&mut self) -> &mut FrozenProd<R> {
        &mut self.frozen
    }
    pub fn release(self) -> CachedProd<R> {
        self.frozen.commit();
        CachedProd { frozen: self.frozen }
    }
}
impl<R: RbRef> FrozenProducer for FrozenCachedProd<R> {
    delegate_frozen_producer!(Self::frozen, Self::frozen_mut);
}

impl<R: RbRef> FrozenCachedCons<R> {
    fn frozen(&self) -> &FrozenCons<R> {
        &self.frozen
    }
    #[allow(dead_code)]
    fn frozen_mut(&mut self) -> &mut FrozenCons<R> {
        &mut self.frozen
    }
    pub fn release(self) -> CachedCons<R> {
        self.frozen.commit();
        CachedCons { frozen: self.frozen }
    }
}
impl<R: RbRef> FrozenConsumer for FrozenCachedCons<R> {
    delegate_frozen_consumer!(Self::frozen, Self::frozen_mut);
}

pub struct FrozenCachedProdRef<'a, R: RbRef> {
    frozen: &'a mut FrozenProd<R>,
}
pub struct FrozenCachedConsRef<'a, R: RbRef> {
    frozen: &'a mut FrozenCons<R>,
}

impl<'a, R: RbRef> FrozenCachedProdRef<'a, R> {
    fn frozen(&self) -> &FrozenProd<R> {
        self.frozen
    }
    fn frozen_mut(&mut self) -> &mut FrozenProd<R> {
        self.frozen
    }
}
impl<'a, R: RbRef> FrozenCachedConsRef<'a, R> {
    fn frozen(&self) -> &FrozenCons<R> {
        self.frozen
    }
    fn frozen_mut(&mut self) -> &mut FrozenCons<R> {
        self.frozen
    }
}
unsafe impl<'a, R: RbRef> Based for FrozenCachedProdRef<'a, R> {
    type Rb = R::Target;
    type RbRef = R;
    fn rb(&self) -> &Self::Rb {
        self.frozen.rb()
    }
    fn rb_ref(&self) -> &Self::RbRef {
        self.frozen.rb_ref()
    }
}
unsafe impl<'a, R: RbRef> Based for FrozenCachedConsRef<'a, R> {
    type Rb = R::Target;
    type RbRef = R;
    fn rb(&self) -> &Self::Rb {
        self.frozen.rb()
    }
    fn rb_ref(&self) -> &Self::RbRef {
        self.frozen.rb_ref()
    }
}

impl<'a, R: RbRef> Observer for FrozenCachedProdRef<'a, R> {
    delegate_observer!(FrozenProd<R>, Self::frozen);
}
impl<'a, R: RbRef> Producer for FrozenCachedProdRef<'a, R> {
    delegate_producer!(Self::frozen, Self::frozen_mut);
}
impl<'a, R: RbRef> FrozenProducer for FrozenCachedProdRef<'a, R> {
    delegate_frozen_producer!(Self::frozen, Self::frozen_mut);
}

impl<'a, R: RbRef> Observer for FrozenCachedConsRef<'a, R> {
    delegate_observer!(FrozenCons<R>, Self::frozen);
}
impl<'a, R: RbRef> Consumer for FrozenCachedConsRef<'a, R> {
    delegate_consumer!(Self::frozen, Self::frozen_mut);
}
impl<'a, R: RbRef> FrozenConsumer for FrozenCachedConsRef<'a, R> {
    delegate_frozen_consumer!(Self::frozen, Self::frozen_mut);
}

impl<'a, R: RbRef> FrozenCachedProdRef<'a, R> {
    #[inline]
    pub fn commit(&self) {
        self.frozen.commit()
    }
    #[inline]
    pub fn sync(&self) {
        self.frozen.sync()
    }
    #[inline]
    pub fn discard(&mut self) {
        self.frozen.discard()
    }
}
impl<'a, R: RbRef> FrozenCachedConsRef<'a, R> {
    #[inline]
    pub fn commit(&self) {
        self.frozen.commit()
    }
    #[inline]
    pub fn sync(&self) {
        self.frozen.sync();
    }
}

impl<'a, R: RbRef> Drop for FrozenCachedProdRef<'a, R> {
    fn drop(&mut self) {
        self.frozen.commit()
    }
}
impl<'a, R: RbRef> Drop for FrozenCachedConsRef<'a, R> {
    fn drop(&mut self) {
        self.frozen.commit()
    }
}
