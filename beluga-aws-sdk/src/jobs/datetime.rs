use std::time::Duration;

use chrono::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Deserializer, Serializer};

pub(super) fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date = date.map(|date| {
        let timestamp = date.timestamp(); // Get the timestamp in whole seconds
        let nanoseconds = date.timestamp_subsec_nanos(); // Get the fractional part (nanoseconds)
                                                         // Calculate the exact time in seconds with milliseconds precision
        let fractional_seconds = nanoseconds as f64 / 1_000_000_000_f64; // Convert nanoseconds to fractional seconds
        timestamp as f64 + fractional_seconds // Combine whole and fractional
                                              // seconds
    });

    if let Some(date) = date {
        serializer.serialize_some::<f64>(&date)
    } else {
        serializer.serialize_none()
    }
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<f64>::deserialize(deserializer).map(|seconds_opt| {
        seconds_opt
            .and_then(|seconds| TimeDelta::from_std(Duration::from_secs_f64(seconds)).ok())
            .and_then(|delta| NaiveDateTime::UNIX_EPOCH.checked_add_signed(delta))
            .map(|datetime| DateTime::from_naive_utc_and_offset(datetime, Utc))
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use serde_json::{from_value, json, to_value};

    use super::*;

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Cell(#[serde(with = "super")] Option<DateTime<Utc>>);

    #[test]
    fn serialize_none() {
        let cell = Cell(None);
        let json_value = to_value(cell).unwrap();
        assert_eq!(json_value, json!(null));
    }

    #[test]
    fn serialize_some() {
        let dt = Utc
            .with_ymd_and_hms(2023, 6, 4, 3, 42, 0)
            .unwrap()
            .with_nanosecond(123_456_789)
            .unwrap();

        let cell = Cell(Some(dt));
        let json_value = to_value(cell).unwrap();
        assert_eq!(json_value, json!(1_685_850_120.123_456_7));
    }

    #[test]
    fn deserialize_none() {
        let cell: Cell = from_value(json!(null)).unwrap();
        assert_eq!(cell.0, None);
    }

    #[test]
    fn deserialize_some() {
        let dt = Utc
            .with_ymd_and_hms(2023, 6, 4, 3, 42, 0)
            .unwrap()
            .with_nanosecond(123_456_789)
            .unwrap();

        let Cell(datetime) = from_value(json!(1_685_850_120.123_456_7)).unwrap();
        assert!(datetime.is_some());
        let datetime = datetime.unwrap();

        assert_eq!(datetime.year(), dt.year());
        assert_eq!(datetime.month(), dt.month());
        assert_eq!(datetime.day(), dt.day());
        assert_eq!(datetime.hour(), dt.hour());
        assert_eq!(datetime.minute(), dt.minute());
        assert_eq!(datetime.second(), dt.second());
        // This ensures that the roundtrip serialization and deserialization of
        // the `Cell` type preserves the nanosecond precision of the original
        // `DateTime` value.
        assert_eq!({ datetime.nanosecond() / 100 }, { dt.nanosecond() / 100 });
    }

    #[test]
    fn roundtrip() {
        let dt = Utc
            .with_ymd_and_hms(2023, 6, 4, 3, 42, 0)
            .unwrap()
            .with_nanosecond(123_456_789)
            .unwrap();

        let cell = Cell(Some(dt));
        let json_value = to_value(cell).unwrap();
        let Cell(datetime) = from_value(json_value).unwrap();
        assert!(datetime.is_some());
        let datetime = datetime.unwrap();

        assert_eq!(datetime.year(), dt.year());
        assert_eq!(datetime.month(), dt.month());
        assert_eq!(datetime.day(), dt.day());
        assert_eq!(datetime.hour(), dt.hour());
        assert_eq!(datetime.minute(), dt.minute());
        assert_eq!(datetime.second(), dt.second());
        // This ensures that the roundtrip serialization and deserialization of
        // the `Cell` type preserves the nanosecond precision of the original
        // `DateTime` value.
        assert_eq!({ datetime.nanosecond() / 100 }, { dt.nanosecond() / 100 });
    }
}
