use crate::traits::{Consumer, Observer, Producer};
use core::ops::Deref;

pub trait BaseRef {
    type Base: Observer;
    fn base_deref(&self) -> &Self::Base;
}
impl<R: Deref> BaseRef for R
where
    R::Target: Observer,
{
    type Base = R::Target;
    fn base_deref(&self) -> &Self::Base {
        self.deref()
    }
}

pub trait ProdRef {
    type Base: Producer;
    fn base_deref(&self) -> &Self::Base;
}
pub trait ConsRef {
    type Base: Consumer;
    fn base_deref(&self) -> &Self::Base;
}

impl<R: BaseRef> ProdRef for R
where
    R::Base: Producer,
{
    type Base = R::Base;
    fn base_deref(&self) -> &Self::Base {
        self.base_deref()
    }
}
impl<R: BaseRef> ConsRef for R
where
    R::Base: Consumer,
{
    type Base = R::Base;
    fn base_deref(&self) -> &Self::Base {
        self.base_deref()
    }
}
