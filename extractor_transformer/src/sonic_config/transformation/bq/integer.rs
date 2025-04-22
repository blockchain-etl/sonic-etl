
use std::convert::Infallible;

use alloy::primitives::Uint;

pub trait IntoInteger {
    fn into_integer(&self) -> i64;

    #[inline]
    fn into_integer_opt(&self) -> Option<i64> {
        Some(self.into_integer())
    }
}

impl IntoInteger for i64 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self
    }
}

impl IntoInteger for i32 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

impl IntoInteger for i16 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

impl IntoInteger for i8 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

impl IntoInteger for u32 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

impl IntoInteger for u16 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

impl IntoInteger for u8 {
    #[inline]
    fn into_integer(&self) -> i64 {
        *self as i64
    }
}

pub trait TryIntoInteger {
    type Error;

    fn try_into_integer(&self) -> Result<i64, Self::Error>;

    #[inline]
    fn try_into_integer_opt(&self) -> Result<Option<i64>, Self::Error> {
        self.try_into_integer().map(Some)
    }
}

impl<T: IntoInteger> TryIntoInteger for T {
    type Error = Infallible;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        Ok(self.into_integer())
    }
}

impl<const BITS: usize, const LIMBS: usize> TryIntoInteger for Uint<BITS, LIMBS> {
    type Error = TryIntoIntegerError<Uint<BITS, LIMBS>>;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        match BITS - self.leading_zeros() {
            ..=63 => {
                let last_limb = self
                    .as_limbs()
                    .last()
                    .expect("Should always be at least one limb");
                match last_limb.try_into_integer() {
                    Ok(integer) => Ok(integer),
                    Err(TryIntoIntegerError::AboveMax(_)) => {
                        Err(TryIntoIntegerError::AboveMax(*self))
                    }
                    Err(TryIntoIntegerError::BelowMin(_)) => unreachable!("Should never be below min considering UINT is unsigned and i64 is less than 0")
                }
            }
            _ => Err(TryIntoIntegerError::AboveMax(*self)),
        }
    }
}

impl TryIntoInteger for u64 {
    type Error = TryIntoIntegerError<u64>;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        if *self > (i64::MAX as u64) {
            Err(TryIntoIntegerError::AboveMax(*self))
        } else {
            Ok(*self as i64)
        }
    }
}

impl TryIntoInteger for u128 {
    type Error = TryIntoIntegerError<u128>;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        if *self > (i64::MAX as u128) {
            Err(TryIntoIntegerError::AboveMax(*self))
        } else {
            Ok(*self as i64)
        }
    }
}

impl TryIntoInteger for i128 {
    type Error = TryIntoIntegerError<i128>;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        if *self > (i64::MAX as i128) {
            Err(TryIntoIntegerError::AboveMax(*self))
        } else {
            Ok(*self as i64)
        }
    }
}

impl TryIntoInteger for usize {
    type Error = TryIntoIntegerError<usize>;
    #[inline]
    fn try_into_integer(&self) -> Result<i64, Self::Error> {
        if *self > (i64::MAX as usize) {
            Err(TryIntoIntegerError::AboveMax(*self))
        } else {
            Ok(*self as i64)
        }
    }
}

#[derive(Debug, Clone)]
pub enum TryIntoIntegerError<T> {
    AboveMax(T),
    #[allow(dead_code)]
    BelowMin(T),
}

impl<T: std::fmt::Debug> std::fmt::Display for TryIntoIntegerError<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to convert into Integer (i64): {:?}", self)
    }
}

impl<T: std::fmt::Debug> std::error::Error for TryIntoIntegerError<T> {}
