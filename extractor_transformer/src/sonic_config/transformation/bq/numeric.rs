use std::convert::Infallible;

use super::integer::IntoInteger;

pub trait IntoNumeric {
    fn into_numeric(&self) -> String;

    #[inline]
    fn into_numeric_opt(&self) -> Option<String> {
        Some(self.into_numeric())
    }
}

impl<T: IntoInteger> IntoNumeric for T {
    #[inline]
    fn into_numeric(&self) -> String {
        self.into_integer().to_string()
    }
}

impl IntoNumeric for u64 {
    fn into_numeric(&self) -> String {
        self.to_string()
    }
}

impl IntoNumeric for usize {
    fn into_numeric(&self) -> String {
        self.to_string()
    }
}

pub trait TryIntoNumeric {
    type Error;

    fn try_into_numeric(&self) -> Result<String, Self::Error>;

    #[inline]
    fn try_into_numeric_opt(&self) -> Result<Option<String>, Self::Error> {
        self.try_into_numeric().map(|item| Some(item))
    }
}

#[derive(Debug, Clone)]
pub enum TryIntoNumericError<T> {
    TooBig(T),
    TooSmall(T),
}

impl<T: std::fmt::Debug> std::fmt::Display for TryIntoNumericError<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to convert into Numeric (String): {:?}", self)
    }
}

impl<T: std::fmt::Debug> std::error::Error for TryIntoNumericError<T> {}

impl<T: IntoNumeric> TryIntoNumeric for T {
    type Error = Infallible;
    #[inline]
    fn try_into_numeric(&self) -> Result<String, Self::Error> {
        Ok(self.into_numeric())
    }
    #[inline]
    fn try_into_numeric_opt(&self) -> Result<Option<String>, Self::Error> {
        Ok(self.into_numeric_opt())
    }
}

const MAX_NUMERIC_I128: i128 = 9999999999999999999999999999;
const MIN_NUMERIC_I128: i128 = -9999999999999999999999999999;

impl TryIntoNumeric for i128 {
    type Error = TryIntoNumericError<i128>;
    #[inline]
    fn try_into_numeric(&self) -> Result<String, Self::Error> {
        if *self < MIN_NUMERIC_I128 {
            return Err(TryIntoNumericError::TooSmall(*self));
        } else if *self > MAX_NUMERIC_I128 {
            return Err(TryIntoNumericError::TooBig(*self));
        } else {
            Ok(self.to_string())
        }
    }
}

const MAX_NUMERIC_U128: u128 = 9999999999999999999999999999;

impl TryIntoNumeric for u128 {
    type Error = TryIntoNumericError<u128>;
    #[inline]
    fn try_into_numeric(&self) -> Result<String, Self::Error> {
        if *self > MAX_NUMERIC_U128 {
            return Err(TryIntoNumericError::TooBig(*self));
        } else {
            Ok(self.to_string())
        }
    }
}
