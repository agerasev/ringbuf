macro_rules! rb_impl_init {
    ($type:ident $(, $param:ident : $bound:ident)*) => {
        impl<T, const N: usize $(, $param: $bound)*> Default for $type<crate::storage::Array<T, N> $(, $param)*> {
            fn default() -> Self {
                unsafe { Self::from_raw_parts(crate::utils::uninit_array().into(), usize::default(), usize::default()) }
            }
        }

        impl<T, const N: usize $(, $param: $bound)*> From<[T; N]> for $type<crate::storage::Array<T, N> $(, $param)*> {
            fn from(value: [T; N]) -> Self {
                $crate::rb::utils::assert_capacity(N);
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::array_to_uninit(value).into(), read, write) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T $(, $param: $bound)*> $type<crate::storage::Heap<T> $(, $param)*> {
            /// Creates a new instance of a ring buffer.
            ///
            /// *Panics if allocation failed, `capacity` is zero, or it exceeds `usize::MAX / 2`.*
            pub fn new(capacity: usize) -> Self {
                $crate::rb::utils::assert_capacity(capacity);
                unsafe { Self::from_raw_parts(crate::storage::Heap::<T>::new(capacity), usize::default(), usize::default()) }
            }
            /// Validate capacity and allocate storage without panicking on either error.
            /// Owned splitting still uses the standard `Arc` allocation behavior.
            pub fn try_new(capacity: usize) -> Result<Self, crate::CreateError> {
                crate::CapacityError::check(capacity).map_err(crate::CreateError::Capacity)?;
                let storage = crate::storage::Heap::<T>::try_new(capacity).map_err(crate::CreateError::Allocation)?;
                Ok(unsafe { Self::from_raw_parts(storage, 0, 0) })
            }

        }

        #[cfg(feature = "alloc")]
        impl<T $(, $param: $bound)*> From<alloc::vec::Vec<T>> for $type<crate::storage::Heap<T> $(, $param)*> {
            fn from(value: alloc::vec::Vec<T>) -> Self {
                $crate::rb::utils::assert_capacity(if core::mem::size_of::<T>() == 0 { value.len() } else { value.capacity() });
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::vec_to_uninit(value).into(), read, write) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T $(, $param: $bound)*> From<alloc::boxed::Box<[T]>> for $type<crate::storage::Heap<T> $(, $param)*> {
            fn from(value: alloc::boxed::Box<[T]>) -> Self {
                $crate::rb::utils::assert_capacity(value.len());
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::boxed_slice_to_uninit(value).into(), read, write) }
            }
        }
    };
}

pub(crate) use rb_impl_init;
