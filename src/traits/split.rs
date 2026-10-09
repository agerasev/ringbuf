use crate::traits::{Consumer, Producer};

/// Split the ring buffer onto producer and consumer.
pub trait Split {
    /// Producer type.
    type Prod: Producer;
    /// Consumer type.
    type Cons: Consumer;

    /// Perform splitting.
    fn split(self) -> (Self::Prod, Self::Cons);
}

/// Split the ring buffer by reference onto producer and consumer.
pub trait SplitRef {
    /// Ref producer type.
    type RefProd<'a>: Producer + 'a
    where
        Self: 'a;
    /// Ref consumer type.
    type RefCons<'a>: Consumer + 'a
    where
        Self: 'a;

    /// Perform splitting by reference.
    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>);
}

/// Select the endpoint adapters returned by the standard split methods.
/// Marker backends use this to provide waiting adapters while reusing `Rb`.
/// Custom policies may also use `endpoint::try_split` directly.
pub trait EndpointPolicy<S: crate::storage::Storage + ?Sized, I: crate::indices::Indices>: crate::markers::Markers {
    type Prod<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>>: Producer;
    type Cons<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>>: Consumer;
    fn wrap_pair<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>>(
        pair: (crate::endpoint::CachedProd<R>, crate::endpoint::CachedCons<R>),
    ) -> (Self::Prod<R>, Self::Cons<R>);
}

macro_rules! immediate_policy {
    ($policy:ty) => {
        impl<S: crate::storage::Storage + ?Sized, I: crate::indices::Indices> EndpointPolicy<S, I> for $policy {
            type Prod<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>> = crate::endpoint::CachedProd<R>;
            type Cons<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>> = crate::endpoint::CachedCons<R>;
            fn wrap_pair<R: crate::rb::RbHandle<Rb = crate::Rb<S, I, Self>>>(
                pair: (Self::Prod<R>, Self::Cons<R>),
            ) -> (Self::Prod<R>, Self::Cons<R>) {
                pair
            }
        }
    };
}
immediate_policy!(crate::markers::AtomicMarkers);
immediate_policy!(crate::markers::LocalMarkers);
immediate_policy!(crate::markers::NoMarkers);
