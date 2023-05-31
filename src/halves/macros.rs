macro_rules! impl_prod_traits {
    ($Prod:ident) => {
        #[cfg(feature = "std")]
        impl<R: $crate::rbs::ref_::RbRef> std::io::Write for $Prod<R>
        where
            R::Target: $crate::traits::Producer<Item = u8>,
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

        impl<R: $crate::rbs::ref_::RbRef> core::fmt::Write for $Prod<R>
        where
            R::Target: $crate::traits::Producer<Item = u8>,
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
        impl<R: $crate::rbs::ref_::RbRef> IntoIterator for $Cons<R>
        where
            R::Target: $crate::traits::Consumer,
        {
            type Item = <R::Target as $crate::traits::Observer>::Item;
            type IntoIter = $crate::consumer::IntoIter<Self>;

            fn into_iter(self) -> Self::IntoIter {
                $crate::consumer::IntoIter(self)
            }
        }

        #[cfg(feature = "std")]
        impl<R: $crate::rbs::ref_::RbRef> std::io::Read for $Cons<R>
        where
            R::Target: $crate::traits::Consumer<Item = u8>,
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
