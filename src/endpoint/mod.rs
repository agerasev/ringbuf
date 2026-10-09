pub mod cached;
pub mod direct;
pub mod frozen;
mod traits;

pub use cached::{Cached, CachedCons, CachedProd};
pub use cached::{CachedCons as Cons, CachedProd as Prod};
pub use direct::Obs;
pub use direct::{Cons as DirectCons, Prod as DirectProd};
#[allow(deprecated)]
pub use frozen::{FrozenCons, FrozenProd};
pub use traits::*;

/// Failed acquisition of endpoint rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireError {
    ProducerHeld,
    ConsumerHeld,
    Untracked,
}

/// Acquire both endpoints through an arbitrary stable handle.
/// On failure any rights acquired by this call are released.
pub fn try_split<R: crate::rb::RbHandle>(rb: R) -> Result<(CachedProd<R>, CachedCons<R>), (AcquireError, R)> {
    let other = rb.clone();
    let prod = direct::Prod::try_new(other).map_err(|(err, _)| (err, rb.clone()))?;
    match direct::Cons::try_new(rb) {
        Ok(cons) => Ok((CachedProd::from_direct(prod), CachedCons::from_direct(cons))),
        Err(err) => {
            drop(prod);
            Err(err)
        }
    }
}

/// Split when ownership of the RB itself proves unique endpoint access.
/// # Safety
/// There must be no active endpoints or data views for this RB. All clones of
/// the handle must be unable to acquire endpoints without ownership checks.
pub unsafe fn split_unchecked<R: crate::rb::RbHandle>(rb: R) -> (CachedProd<R>, CachedCons<R>) {
    let other = rb.clone();
    unsafe {
        (
            CachedProd::from_direct(direct::Prod::new_unchecked(other)),
            CachedCons::from_direct(direct::Cons::new_unchecked(rb)),
        )
    }
}

pub use cached as caching;
pub use cached::{CachedCons as CachingCons, CachedProd as CachingProd};
pub use traits::Endpoint as Wrap;

pub mod deferred;
pub use deferred::{Deferred, DeferredCons, DeferredProd};
