//! Compatibility names for deferred endpoints.
use super::{Cached, Deferred};
#[deprecated(note = "use Deferred")]
pub type Frozen<R, const P: bool, const C: bool> = Deferred<Cached<R, P, C>, P, C>;
#[deprecated(note = "use DeferredProd")]
pub type FrozenProd<R> = super::DeferredProd<super::CachedProd<R>>;
#[deprecated(note = "use DeferredCons")]
pub type FrozenCons<R> = super::DeferredCons<super::CachedCons<R>>;
