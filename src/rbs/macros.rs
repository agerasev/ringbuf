macro_rules! rb_impl_init {
    ($type:ident) => {
        impl<T, const N: usize> Default for $type<Static<T, N>> {
            fn default() -> Self {
                unsafe { Self::from_raw_parts(crate::utils::uninit_array(), usize::default(), usize::default()) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> $type<Heap<T>> {
            /// Creates a new instance of a ring buffer.
            ///
            /// *Panics if allocation failed or `capacity` is zero.*
            pub fn new(capacity: usize) -> Self {
                Self::try_new(capacity).unwrap()
            }
            /// Creates a new instance of a ring buffer returning an error if allocation failed.
            ///
            /// *Panics if `capacity` is zero.*
            pub fn try_new(capacity: usize) -> Result<Self, alloc::collections::TryReserveError> {
                let mut data = alloc::vec::Vec::new();
                data.try_reserve_exact(capacity)?;
                data.resize_with(capacity, core::mem::MaybeUninit::uninit);
                Ok(unsafe { Self::from_raw_parts(data, usize::default(), usize::default()) })
            }
        }
    };
}

pub(crate) use rb_impl_init;
