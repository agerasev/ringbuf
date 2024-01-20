macro_rules! rb_impl_init {
    ($type:ident) => {
        impl<T, const N: usize> Default for $type<crate::storage::Array<T, N>> {
            fn default() -> Self {
                unsafe { Self::from_raw_parts(crate::utils::uninit_array().into(), usize::default(), usize::default()) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> $type<crate::storage::Heap<T>> {
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
                let mut vec = alloc::vec::Vec::new();
                vec.try_reserve_exact(capacity)?;
                let ptr = vec.as_mut_ptr();
                core::mem::forget(vec);
                let data = unsafe { Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, capacity)) };
                assert_eq!(data.len(), capacity);
                assert_eq!(ptr as *const _, data.as_ptr());
                Ok(unsafe { Self::from_raw_parts(data.into(), usize::default(), usize::default()) })
            }
        }
    };
}

pub(crate) use rb_impl_init;
