use super::integer::{TryIntoInteger, TryIntoIntegerError};

pub trait IntoTimestamp {
    fn into_timestamp(&self) -> i64;

    #[inline]
    fn into_timestamp_opt(&self) -> Option<i64> {
        Some(self.into_timestamp())
    }
}

pub trait TryIntoTimestamp {
    type Error;

    fn try_into_timestamp(&self) -> Result<i64, Self::Error>;

    #[inline]
    fn try_into_timestamp_opt(&self) -> Result<Option<i64>, Self::Error> {
        self.try_into_timestamp().map(|ts| Some(ts))
    }
}

impl TryIntoTimestamp for u64 {
    type Error = TryIntoIntegerError<u64>;
    #[inline]
    fn try_into_timestamp(&self) -> Result<i64, Self::Error> {
        self.try_into_integer()
    }
}
