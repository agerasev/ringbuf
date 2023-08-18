mod cons;
mod prod;

use crate::rb::AsyncRbRef;
use core::{mem::MaybeUninit, num::NonZeroUsize};
use ringbuf::{rb::traits::ToRbRef, traits::Observer, wrap::caching::Caching, Obs};

pub struct AsyncWrap<R: AsyncRbRef, const P: bool, const C: bool> {
    base: Option<Caching<R, P, C>>,
}

pub type AsyncProd<R> = AsyncWrap<R, true, false>;
pub type AsyncCons<R> = AsyncWrap<R, false, true>;

impl<R: AsyncRbRef, const P: bool, const C: bool> AsyncWrap<R, P, C> {
    pub unsafe fn new(rb: R) -> Self {
        Self {
            base: Some(Caching::new(rb)),
        }
    }

    fn base(&self) -> &Caching<R, P, C> {
        self.base.as_ref().unwrap()
    }
    fn base_mut(&mut self) -> &mut Caching<R, P, C> {
        self.base.as_mut().unwrap()
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}

impl<R: AsyncRbRef, const P: bool, const C: bool> ToRbRef for AsyncWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        self.base().rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.base.unwrap().into_rb_ref()
    }
}

impl<R: AsyncRbRef, const P: bool, const C: bool> Unpin for AsyncWrap<R, P, C> {}

impl<R: AsyncRbRef, const P: bool, const C: bool> Observer for AsyncWrap<R, P, C> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base().capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.base().read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.base().write_index()
    }
    #[inline]
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.base().unsafe_slices(start, end)
    }

    #[inline]
    fn read_is_held(&self) -> bool {
        self.base().read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.base().write_is_held()
    }
}
