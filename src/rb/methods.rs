/// Implement inherent data-access methods for a type implementing
/// [`crate::traits::RingBuffer`]. All data views borrow the buffer exclusively.
///
/// The second argument lists the implementation's generic parameters.
#[macro_export]
macro_rules! impl_ring_buffer_methods {
    ($ty:ty, [$($generics:tt)*]) => {
        impl<$($generics)*> $ty {
            #[doc = "See [`RingBuffer::as_exclusive`]."]
            #[inline]
            pub fn as_exclusive(&mut self) -> $crate::rb::Exclusive<'_, Self> {
                $crate::traits::RingBuffer::as_exclusive(self)
            }
            #[doc = "See [`RingBuffer::occupied_slices`]."]
            #[inline]
            pub fn occupied_slices(
                &mut self,
            ) -> (
                &[core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
                &[core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
            ) {
                $crate::traits::RingBuffer::occupied_slices(self)
            }
            #[doc = "See [`RingBuffer::as_slices`]."]
            #[inline]
            pub fn as_slices(
                &mut self,
            ) -> (
                &[<Self as $crate::traits::Observer>::Item],
                &[<Self as $crate::traits::Observer>::Item],
            ) {
                $crate::traits::RingBuffer::as_slices(self)
            }
            #[doc = "See [`RingBuffer::as_mut_slices`]."]
            #[inline]
            pub fn as_mut_slices(
                &mut self,
            ) -> (
                &mut [<Self as $crate::traits::Observer>::Item],
                &mut [<Self as $crate::traits::Observer>::Item],
            ) {
                $crate::traits::RingBuffer::as_mut_slices(self)
            }
            #[doc = "See [`RingBuffer::first`]."]
            #[inline]
            pub fn first(&mut self) -> Option<&<Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::first(self)
            }
            #[doc = "See [`RingBuffer::first_mut`]."]
            #[inline]
            pub fn first_mut(&mut self) -> Option<&mut <Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::first_mut(self)
            }
            #[doc = "See [`RingBuffer::last`]."]
            #[inline]
            pub fn last(&mut self) -> Option<&<Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::last(self)
            }
            #[doc = "See [`RingBuffer::last_mut`]."]
            #[inline]
            pub fn last_mut(&mut self) -> Option<&mut <Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::last_mut(self)
            }
            #[doc = "See [`RingBuffer::try_peek`]."]
            #[inline]
            pub fn try_peek(&mut self) -> Option<&<Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::try_peek(self)
            }
            #[doc = "See [`RingBuffer::try_pop`]."]
            #[inline]
            pub fn try_pop(&mut self) -> Option<<Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::try_pop(self)
            }
            #[doc = "See [`RingBuffer::peek_slice_uninit`]."]
            #[inline]
            pub fn peek_slice_uninit(&mut self, elems: &mut [core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>]) -> usize {
                $crate::traits::RingBuffer::peek_slice_uninit(self, elems)
            }
            #[doc = "See [`RingBuffer::peek_slice`]."]
            #[inline]
            pub fn peek_slice(&mut self, elems: &mut [<Self as $crate::traits::Observer>::Item]) -> usize
            where
                <Self as $crate::traits::Observer>::Item: Copy,
            {
                $crate::traits::RingBuffer::peek_slice(self, elems)
            }
            #[doc = "See [`RingBuffer::pop_slice_uninit`]."]
            #[inline]
            pub fn pop_slice_uninit(&mut self, elems: &mut [core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>]) -> usize {
                $crate::traits::RingBuffer::pop_slice_uninit(self, elems)
            }
            #[doc = "See [`RingBuffer::pop_slice`]."]
            #[inline]
            pub fn pop_slice(&mut self, elems: &mut [<Self as $crate::traits::Observer>::Item]) -> usize
            where
                <Self as $crate::traits::Observer>::Item: Copy,
            {
                $crate::traits::RingBuffer::pop_slice(self, elems)
            }
            #[doc = "See [`RingBuffer::pop_iter`]."]
            #[inline]
            pub fn pop_iter(&mut self) -> $crate::traits::consumer::PopIter<'_, Self> {
                $crate::traits::RingBuffer::pop_iter(self)
            }
            #[doc = "See [`RingBuffer::iter`]."]
            #[inline]
            pub fn iter(&mut self) -> $crate::traits::consumer::Iter<'_, Self> {
                $crate::traits::RingBuffer::iter(self)
            }
            #[doc = "See [`RingBuffer::iter_mut`]."]
            #[inline]
            pub fn iter_mut(&mut self) -> $crate::traits::consumer::IterMut<'_, Self> {
                $crate::traits::RingBuffer::iter_mut(self)
            }
            #[doc = "See [`RingBuffer::skip`]."]
            #[inline]
            pub fn skip(&mut self, count: usize) -> usize {
                $crate::traits::RingBuffer::skip(self, count)
            }
            #[doc = "See [`RingBuffer::clear`]."]
            #[inline]
            pub fn clear(&mut self) -> usize {
                $crate::traits::RingBuffer::clear(self)
            }
            #[doc = "See [`RingBuffer::vacant_slices`]."]
            #[inline]
            pub fn vacant_slices(
                &mut self,
            ) -> (
                &[core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
                &[core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
            ) {
                $crate::traits::RingBuffer::vacant_slices(self)
            }
            #[doc = "See [`RingBuffer::vacant_slices_mut`]."]
            #[inline]
            pub fn vacant_slices_mut(
                &mut self,
            ) -> (
                &mut [core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
                &mut [core::mem::MaybeUninit<<Self as $crate::traits::Observer>::Item>],
            ) {
                $crate::traits::RingBuffer::vacant_slices_mut(self)
            }
            #[doc = "See [`RingBuffer::try_push`]."]
            #[inline]
            pub fn try_push(&mut self, elem: <Self as $crate::traits::Observer>::Item) -> Result<(), <Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::try_push(self, elem)
            }
            #[doc = "See [`RingBuffer::push_iter`]."]
            #[inline]
            pub fn push_iter<I: Iterator<Item = <Self as $crate::traits::Observer>::Item>>(&mut self, iter: I) -> usize {
                $crate::traits::RingBuffer::push_iter(self, iter)
            }
            #[doc = "See [`RingBuffer::push_slice`]."]
            #[inline]
            pub fn push_slice(&mut self, elems: &[<Self as $crate::traits::Observer>::Item]) -> usize
            where
                <Self as $crate::traits::Observer>::Item: Copy,
            {
                $crate::traits::RingBuffer::push_slice(self, elems)
            }
            #[cfg(feature = "std")]
            #[doc = "See [`RingBuffer::read_from`]."]
            #[inline]
            pub fn read_from<Input: std::io::Read>(&mut self, reader: &mut Input, count: Option<usize>) -> Option<std::io::Result<usize>>
            where
                Self: $crate::traits::Observer<Item = u8>,
            {
                $crate::traits::RingBuffer::read_from(self, reader, count)
            }
            #[cfg(feature = "std")]
            #[doc = "See [`RingBuffer::write_into`]."]
            #[inline]
            pub fn write_into<Output: std::io::Write>(&mut self, writer: &mut Output, count: Option<usize>) -> Option<std::io::Result<usize>>
            where
                Self: $crate::traits::Observer<Item = u8>,
            {
                $crate::traits::RingBuffer::write_into(self, writer, count)
            }
            #[doc = "See [`RingBuffer::push_overwrite`]."]
            #[inline]
            pub fn push_overwrite(&mut self, elem: <Self as $crate::traits::Observer>::Item) -> Option<<Self as $crate::traits::Observer>::Item> {
                $crate::traits::RingBuffer::push_overwrite(self, elem)
            }
            #[doc = "See [`RingBuffer::push_iter_overwrite`]."]
            #[inline]
            pub fn push_iter_overwrite<I: Iterator<Item = <Self as $crate::traits::Observer>::Item>>(&mut self, iter: I) {
                $crate::traits::RingBuffer::push_iter_overwrite(self, iter)
            }
            #[doc = "See [`RingBuffer::push_slice_overwrite`]."]
            #[inline]
            pub fn push_slice_overwrite(&mut self, elems: &[<Self as $crate::traits::Observer>::Item])
            where
                <Self as $crate::traits::Observer>::Item: Copy,
            {
                $crate::traits::RingBuffer::push_slice_overwrite(self, elems)
            }
        }
    };
}
