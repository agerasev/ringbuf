use super::Observer;
use core::num::NonZeroUsize;

/// Trait that should be implemented by ring buffer wrappers.
///
/// Used for automatically delegating methods.
pub trait Delegate {
    /// Type the wrapper based on.
    type Base: ?Sized;
    /// Reference to base.
    fn base(&self) -> &Self::Base;
    /// Mutable reference to base.
    fn base_mut(&mut self) -> &mut Self::Base;
}

/// Modulus for pointers to item in ring buffer storage.
///
/// Equals to `2 * capacity`.
#[inline]
pub fn modulus<O: Observer + ?Sized>(this: &O) -> NonZeroUsize {
    this.capacity()
        .checked_mul(NonZeroUsize::new(2).unwrap())
        .expect("capacity exceeds usize::MAX / 2")
}

/// Add two values below `modulus` without overflowing the intermediate sum.
#[inline]
pub fn add_mod(left: usize, right: usize, modulus: NonZeroUsize) -> usize {
    debug_assert!(left < modulus.get() && right < modulus.get());
    if modulus.get().is_power_of_two() {
        // Preserve the bitmask fast path for power-of-two capacities.
        left.wrapping_add(right) & (modulus.get() - 1)
    } else {
        let until_wrap = modulus.get() - left;
        if right >= until_wrap { right - until_wrap } else { left + right }
    }
}

/// Subtract two values below `modulus` without underflowing.
#[inline]
pub fn sub_mod(left: usize, right: usize, modulus: NonZeroUsize) -> usize {
    debug_assert!(left < modulus.get() && right < modulus.get());
    if left >= right {
        left - right
    } else {
        modulus.get() - (right - left)
    }
}

#[cfg(test)]
mod tests {
    use super::{add_mod, sub_mod};
    use core::num::NonZeroUsize;

    fn check(left: usize, right: usize, modulus: usize) {
        let (a, b, m) = (left as u128, right as u128, modulus as u128);
        let modulus = NonZeroUsize::new(modulus).unwrap();
        assert_eq!(add_mod(left, right, modulus), ((a + b) % m) as usize);
        assert_eq!(sub_mod(left, right, modulus), ((m + a - b) % m) as usize);
    }

    #[test]
    fn modular_arithmetic() {
        for modulus in 1..=8 {
            for left in 0..modulus {
                for right in 0..modulus {
                    check(left, right, modulus);
                }
            }
        }
        for modulus in [usize::MAX / 2, usize::MAX / 2 + 1, usize::MAX - 1, usize::MAX] {
            for left in [0, 1, modulus / 2, modulus - 2, modulus - 1] {
                for right in [0, 1, modulus / 2, modulus - 2, modulus - 1] {
                    check(left, right, modulus);
                }
            }
        }
    }
}
