use chrono::{
    format::{ParseError, ParseErrorKind},
    DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc,
};

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

pub fn format_datetime(timestamp: i64, fmt: &str) -> String {
    DateTime::<Utc>::from(Timestamp(timestamp))
        .format(fmt)
        .to_string()
}

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

pub fn int_to_datetime(i: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from(Timestamp::from(i))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        parse_datetime("2024-04-10", "%Y-%m-%d").unwrap();
    }
}
