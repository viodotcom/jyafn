//! Utilities for this crate.
pub(crate) mod murmur;

use std::ffi::CString;

use chrono::{
    format::{ParseError, ParseErrorKind},
    DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc,
};

/// Parses a datetime from string, given a format string and converts the result into the
/// UTC timezone.
pub fn parse_datetime(s: &str, fmt: &str) -> chrono::ParseResult<DateTime<Utc>> {
    fn enough<T>(o: &Result<T, ParseError>) -> bool {
        !matches!(
            o.as_ref().err().map(ParseError::kind),
            Some(ParseErrorKind::NotEnough)
        )
    }

    // Try with timezone first:
    let outcome = DateTime::<FixedOffset>::parse_from_str(s, fmt).map(|d| d.to_utc());
    if enough(&outcome) {
        return outcome;
    }

    // If the error is "not enough", let's try _naive_ and convert to UTC.
    let outcome = NaiveDateTime::parse_from_str(s, fmt).map(|n| n.and_utc());
    if enough(&outcome) {
        return outcome;
    }

    // If this is still not enough, let try date:
    let outcome = NaiveDate::parse_from_str(s, fmt).map(|n| n.and_time(NaiveTime::MIN).and_utc());
    if enough(&outcome) {
        return outcome;
    }

    // Lastly, try naive time and put it at the Unix epoch:
    NaiveTime::parse_from_str(s, fmt)
        .map(|t| NaiveDateTime::UNIX_EPOCH.date().and_time(t).and_utc())
}

/// Formats a raw timestamp with the supplied format into a string.
pub fn format_datetime(timestamp: i64, fmt: &str) -> String {
    DateTime::<Utc>::from(Timestamp(timestamp))
        .format(fmt)
        .to_string()
}

/// Holds a raw timestamp. This type is used for safe conversion from and to `i64` and [`DateTime`].
pub struct Timestamp(i64);

impl From<DateTime<Utc>> for Timestamp {
    fn from(datetime: DateTime<Utc>) -> Timestamp {
        Timestamp(datetime.timestamp_micros())
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(timestamp: Timestamp) -> DateTime<Utc> {
        DateTime::from_timestamp_micros(timestamp.0).expect("out of range timestamp")
    }
}

impl From<Timestamp> for i64 {
    fn from(timestamp: Timestamp) -> i64 {
        timestamp.0
    }
}

impl From<i64> for Timestamp {
    fn from(int: i64) -> Timestamp {
        Timestamp(int)
    }
}

/// Tranforms an integer into a datetime in UTC.
pub fn int_to_datetime(i: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from(Timestamp::from(i))
}

/// Creates a C-style string out of a `String` in a way that doesn't produce errors. This
/// function substitutes nul characters by the ` ` (space) character. This avoids an
/// allocation.
///
/// This method **leaks** the string. So, don't forget to guarantee that somene somewhere
/// is freeing it.
///
/// # Note
///
/// Yes, I know! It's a pretty lousy implementation that is even... O(n^2) (!!). You can
/// do better than I in 10mins.
pub(crate) fn make_safe_c_str(s: String) -> CString {
    let mut v = s.into_bytes();
    loop {
        match std::ffi::CString::new(v) {
            Ok(c_str) => return c_str,
            Err(err) => {
                let nul_position = err.nul_position();
                v = err.into_vec();
                v[nul_position] = b' ';
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        parse_datetime("2024-04-10", "%Y-%m-%d").unwrap();
    }
}
