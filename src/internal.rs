#![allow(dead_code)]

/// Represents an integer type that can be converted into -1.
pub trait MinusOne {
    fn minus_one() -> Self;
}

/// Represents a signed integer type that can be converted into -1.
pub trait MinusOneSigned: MinusOne {}

/// Represents an unsigned integer type that can be converted into
/// -1.
///
/// Obviously, unsigned integers cannot store -1 natively; hence,
/// this usually practically translates to T::MAX.
pub trait MinusOneUnsigned: MinusOne {}

macro_rules! i_minus_one {
    ($t:ty) => {
        impl MinusOne for $t {
            #[inline(always)]
            fn minus_one() -> Self {
                -1
            }
        }

        impl MinusOneSigned for $t {}
    };
}

i_minus_one!(isize);
i_minus_one!(i8);
i_minus_one!(i16);
i_minus_one!(i32);
i_minus_one!(i64);
i_minus_one!(i128);

// For unsigned types, we cast -1 to the corresponding signed type
// and then cast it back to the unsigned type.

macro_rules! u_minus_one {
    ($ut:ty, $it:ty) => {
        impl MinusOne for $ut {
            #[inline(always)]
            fn minus_one() -> Self {
                ((-1) as $it) as Self
            }
        }

        impl MinusOneUnsigned for $ut {}
    };
}

u_minus_one!(usize, isize);
u_minus_one!(u8, i8);
u_minus_one!(u16, i16);
u_minus_one!(u32, i32);
u_minus_one!(u64, i64);
u_minus_one!(u128, i128);

/// Returns the -1 value for the given type, whether it is signed
/// or unsigned.
#[inline(always)]
pub fn minus_one_either<T: MinusOne>() -> T {
    T::minus_one()
}

/// Returns the -1 value for the given signed type.
#[inline(always)]
pub fn minus_one_signed<T: MinusOneSigned>() -> T {
    T::minus_one()
}

/// Returns the -1 value for the given unsigned type.
///
/// Obviously, unsigned integers cannot store -1 natively; hence,
/// this usually practically translates to T::MAX.
#[inline(always)]
pub fn minus_one_unsigned<T: MinusOneUnsigned>() -> T {
    T::minus_one()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minus_one_either() {
        assert_eq!(minus_one_either::<i8>(), -1);
        assert_eq!(minus_one_either::<i16>(), -1);
        assert_eq!(minus_one_either::<i32>(), -1);
        assert_eq!(minus_one_either::<i64>(), -1);
        assert_eq!(minus_one_either::<i128>(), -1);
        assert_eq!(minus_one_either::<isize>(), -1);

        assert_eq!(minus_one_either::<u8>(), (-1i8) as u8);
        assert_eq!(minus_one_either::<u16>(), (-1i16) as u16);
        assert_eq!(minus_one_either::<u32>(), (-1i32) as u32);
        assert_eq!(minus_one_either::<u64>(), (-1i64) as u64);
        assert_eq!(minus_one_either::<u128>(), (-1i128) as u128);
        assert_eq!(minus_one_either::<usize>(), (-1isize) as usize);
    }

    #[test]
    fn test_minus_one_signed() {
        assert_eq!(minus_one_signed::<i8>(), -1);
        assert_eq!(minus_one_signed::<i16>(), -1);
        assert_eq!(minus_one_signed::<i32>(), -1);
        assert_eq!(minus_one_signed::<i64>(), -1);
        assert_eq!(minus_one_signed::<i128>(), -1);
        assert_eq!(minus_one_signed::<isize>(), -1);
    }

    #[test]
    fn test_minus_one_unsigned() {
        assert_eq!(minus_one_unsigned::<u8>(), (-1i8) as u8);
        assert_eq!(minus_one_unsigned::<u16>(), (-1i16) as u16);
        assert_eq!(minus_one_unsigned::<u32>(), (-1i32) as u32);
        assert_eq!(minus_one_unsigned::<u64>(), (-1i64) as u64);
        assert_eq!(minus_one_unsigned::<u128>(), (-1i128) as u128);
        assert_eq!(minus_one_unsigned::<usize>(), (-1isize) as usize);
    }
}
