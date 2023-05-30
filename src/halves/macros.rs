macro_rules! impl_prod_traits {
    ($Prod:ident) => {
        #[cfg(feature = "std")]
        impl<R: $crate::halves::Based> std::io::Write for $Prod<R>
        where
            R::Base: $crate::traits::Producer<Item = u8>,
        {
            fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
                use $crate::producer::Producer;
                let n = self.push_slice(buffer);
                if n == 0 && !buffer.is_empty() {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<R: $crate::halves::Based> core::fmt::Write for $Prod<R>
        where
            R::Base: $crate::traits::Producer<Item = u8>,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use $crate::producer::Producer;
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
pub(crate) use impl_prod_traits;

macro_rules! impl_cons_traits {
    ($Cons:ident) => {
        impl<R: $crate::halves::Based> IntoIterator for $Cons<R>
        where
            R::Base: $crate::traits::Consumer,
        {
            type Item = <R::Base as $crate::traits::Observer>::Item;
            type IntoIter = $crate::consumer::IntoIter<Self>;

            fn into_iter(self) -> Self::IntoIter {
                $crate::consumer::IntoIter(self)
            }
        }

        #[cfg(feature = "std")]
        impl<R: $crate::halves::Based> std::io::Read for $Cons<R>
        where
            R::Base: $crate::traits::Consumer<Item = u8>,
        {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                use $crate::consumer::Consumer;
                let n = self.pop_slice(buffer);
                if n == 0 && !buffer.is_empty() {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
        }
    };
}
pub(crate) use impl_cons_traits;

macro_rules! impl_prod_freeze {
    ($Prod:ident) => {
        impl<R: $crate::halves::Based> $Prod<R>
        where
            R::Base: $crate::traits::Producer,
        {
            pub fn freeze(&mut self) -> $crate::halves::FrozenProd<&Self> {
                unsafe { $crate::halves::FrozenProd::new(self) }
            }
            pub fn into_frozen(self) -> $crate::halves::FrozenProd<Self> {
                unsafe { $crate::halves::FrozenProd::new(self) }
            }
        }
    };
}
pub(crate) use impl_prod_freeze;

macro_rules! impl_cons_freeze {
    ($Cons:ident) => {
        impl<R: $crate::halves::Based> $Cons<R>
        where
            R::Base: $crate::traits::Consumer,
        {
            pub fn freeze(&mut self) -> $crate::halves::FrozenCons<&Self> {
                unsafe { $crate::halves::FrozenCons::new(self) }
            }
            pub fn into_frozen(self) -> $crate::halves::FrozenCons<Self> {
                unsafe { $crate::halves::FrozenCons::new(self) }
            }
        }
    };
}
pub(crate) use impl_cons_freeze;
