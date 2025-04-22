use std::convert::Infallible;

use super::numeric::IntoNumeric;

pub trait IntoBigNumeric {
    fn into_bignumeric(&self) -> String;

    #[inline]
    fn into_bignumeric_opt(&self) -> Option<String> {
        Some(self.into_bignumeric())
    }
}

impl<T: IntoNumeric> IntoBigNumeric for T {
    #[inline]
    fn into_bignumeric(&self) -> String {
        self.into_numeric()
    }
    #[inline]
    fn into_bignumeric_opt(&self) -> Option<String> {
        self.into_numeric_opt()
    }
}

impl IntoBigNumeric for u128 {
    #[inline]
    fn into_bignumeric(&self) -> String {
        self.to_string()
    }
}

impl IntoBigNumeric for i128 {
    #[inline]
    fn into_bignumeric(&self) -> String {
        self.to_string()
    }
}

pub trait TryIntoBigNumeric {
    type Error;

    fn try_into_bignumeric(&self) -> Result<String, Self::Error>;

    #[inline]
    fn try_into_bignumeric_opt(&self) -> Result<Option<String>, Self::Error> {
        self.try_into_bignumeric().map(Some)
    }
}
