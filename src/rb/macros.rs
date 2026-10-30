macro_rules! rb_impl_init {
    ($type:ident) => {
        impl<T, const N: usize> Default for $type<crate::storage::Array<T, N>> {
            fn default() -> Self {
                unsafe { Self::from_raw_parts(crate::utils::uninit_array().into(), usize::default(), usize::default()) }
            }
        }

        impl<T, const N: usize> From<[T; N]> for $type<crate::storage::Array<T, N>> {
            fn from(value: [T; N]) -> Self {
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::array_to_uninit(value).into(), read, write) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> $type<crate::storage::Heap<T>> {
            /// Creates a new instance of a ring buffer.
            ///
            /// *Panics if allocation failed, `capacity` is zero, or it exceeds `usize::MAX / 2`.*
            pub fn new(capacity: usize) -> Self {
                $crate::rb::utils::assert_capacity(capacity);
                unsafe { Self::from_raw_parts(crate::storage::Heap::<T>::new(capacity), usize::default(), usize::default()) }
            }
            /// Creates a new instance of a ring buffer returning an error if allocation failed.
            ///
            /// *Panics if `capacity` is zero or exceeds `usize::MAX / 2`.*
            pub fn try_new(capacity: usize) -> Result<Self, alloc::collections::TryReserveError> {
                $crate::rb::utils::assert_capacity(capacity);
                let mut vec = alloc::vec::Vec::<core::mem::MaybeUninit<T>>::new();
                vec.try_reserve_exact(capacity)?;
                unsafe { vec.set_len(capacity) };
                Ok(unsafe { Self::from_raw_parts(vec.into_boxed_slice().into(), usize::default(), usize::default()) })
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> From<alloc::vec::Vec<T>> for $type<crate::storage::Heap<T>> {
            fn from(value: alloc::vec::Vec<T>) -> Self {
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::vec_to_uninit(value).into(), read, write) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> From<alloc::boxed::Box<[T]>> for $type<crate::storage::Heap<T>> {
            fn from(value: alloc::boxed::Box<[T]>) -> Self {
                let (read, write) = (0, value.len());
                unsafe { Self::from_raw_parts(crate::utils::boxed_slice_to_uninit(value).into(), read, write) }
            }
        }
    };
}

pub(crate) use rb_impl_init;

macro_rules! rb_impl_producer_traits {
    ($type:ident $(< $( $param:tt $( : $first_bound:tt $(+ $next_bound:tt )* )? ),+ >)?) => {

        #[cfg(feature = "std")]
        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? std::io::Write for $type $(< $( $param ),+ >)?
        where
            Self: $crate::traits::RingBuffer<Item = u8>,
        {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let n = self.push_slice(buf);
                if n == 0 {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
         }

        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? core::fmt::Write for $type $(< $( $param ),+ >)?
        where
            Self: $crate::traits::RingBuffer<Item = u8>,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let n = self.push_slice(s.as_bytes());
                if n != s.len() {
                    Err(core::fmt::Error::default())
                } else {
                    Ok(())
                }
            }
         }
    };
 }
pub(crate) use rb_impl_producer_traits;

macro_rules! rb_impl_consumer_traits {
    ($type:ident $(< $( $param:tt $( : $first_bound:tt $(+ $next_bound:tt )* )? ),+ >)?) => {
        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? core::iter::IntoIterator for $type $(< $( $param ),+ >)? where Self: Sized {
            type Item = <Self as $crate::traits::Observer>::Item;
            type IntoIter = $crate::rb::IntoIter<Self>;
            fn into_iter(self) -> Self::IntoIter {
                $crate::rb::IntoIter::new(self)
            }
        }

        #[cfg(feature = "std")]
        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? std::io::Read for $type $(< $( $param ),+ >)?
        where
            Self: $crate::traits::RingBuffer<Item = u8>,
        {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.pop_slice(buf);
                if n == 0 {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
        }
    };
}
pub(crate) use rb_impl_consumer_traits;
