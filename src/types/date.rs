use std::fmt::{Debug, Display};

use chrono::{Datelike, Days, NaiveDate};
use pyo3::{prelude::*, types::PyDate};
use serde::{Deserialize, Serialize};

/// A simple date.
///
/// Dates are stored as [`chrono::NaiveDate`].
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Date(NaiveDate);

const ONE_DAY: Days = Days::new(1);

pub const MIN_DATE: Date = Date(NaiveDate::MIN);

impl Date {
    #[must_use]
    /// Construct a date from year, month, and day.
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, day).map(Self)
    }

    /// Try to parse a date from a string like "2012-12-12".
    pub(crate) fn try_from_str(s: &str) -> Result<Self, ()> {
        if s.len() < 10 {
            return Err(());
        }
        Ok(Self(
            NaiveDate::from_ymd_opt(
                s[0..4].parse().map_err(|_| ())?,
                s[5..7].parse().map_err(|_| ())?,
                s[8..10].parse().map_err(|_| ())?,
            )
            .ok_or(())?,
        ))
    }

    /// Get the year of this date.
    #[must_use]
    pub fn year(self) -> i32 {
        self.0.year()
    }

    /// Get the month of this date.
    #[must_use]
    pub fn month(self) -> u32 {
        self.0.month()
    }

    /// Get the day of this date.
    #[must_use]
    pub fn day(self) -> u32 {
        self.0.day()
    }

    /// Get the day previous to this day.
    #[must_use]
    pub fn previous_day(self) -> Option<Self> {
        self.0.checked_sub_days(ONE_DAY).map(Self)
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}",
            self.year(),
            self.month(),
            self.day()
        )
    }
}

impl Debug for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Date").field(&self.to_string()).finish()
    }
}

impl<'py> IntoPyObject<'py> for &Date {
    type Target = PyDate;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    #[allow(clippy::cast_possible_truncation)]
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        PyDate::new(py, self.year(), self.month() as u8, self.day() as u8)
    }
}

impl<'py> FromPyObject<'_, 'py> for Date {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let py = obj.py();
        let year = obj.getattr(pyo3::intern!(py, "year"))?.extract()?;
        let month = obj.getattr(pyo3::intern!(py, "month"))?.extract()?;
        let day = obj.getattr(pyo3::intern!(py, "day"))?.extract()?;
        Ok(Self(
            NaiveDate::from_ymd_opt(year, month, day).expect("Python date to be a valid date."),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn date_getters() {
        let date = Date::try_from_str("2023-01-03").unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 3);
        assert_eq!(date.to_string(), "2023-01-03");
    }

    #[test]
    fn date_from_str() {
        assert!(Date::try_from_str("2022").is_err());
        assert!(Date::try_from_str("2022-12-1").is_err());
        assert!(Date::try_from_str("2022-22-11").is_err());
        let date = Date::try_from_str("2022-12-12").unwrap();
        assert_eq!((date.year(), date.month(), date.day()), (2022, 12, 12));
        let date = Date::try_from_str("2022-12-12aasdfasdfasdf").unwrap();
        assert_eq!((date.year(), date.month(), date.day()), (2022, 12, 12));
    }

    #[test]
    fn date_from_str_invalid_parse() {
        assert!(Date::try_from_str("abcd-12-12").is_err());
        assert!(Date::try_from_str("2022-ab-12").is_err());
        assert!(Date::try_from_str("2022-12-ab").is_err());
        assert!(Date::try_from_str("2022-02-30").is_err());
    }

    #[test]
    fn date_serialisation() {
        let date = serde_json::from_str::<Date>("\"2022-12-12\"").unwrap();
        assert_eq!(serde_json::to_string(&date).unwrap(), "\"2022-12-12\"");
        assert!(serde_json::from_str::<Date>("\"2022\"").is_err());
        assert!(serde_json::from_str::<Date>("\"2022-12-111\"").is_err());
    }

    #[test]
    fn date_from_ymd_opt() {
        let date = Date::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);

        assert!(Date::from_ymd_opt(2023, 2, 30).is_none());
    }

    #[test]
    fn date_previous_day() {
        let date = Date::from_ymd_opt(2023, 6, 15).unwrap();
        let prev = date.previous_day().unwrap();
        assert_eq!(prev.to_string(), "2023-06-14");

        let date = Date::from_ymd_opt(2023, 3, 1).unwrap();
        let prev = date.previous_day().unwrap();
        assert_eq!(prev.to_string(), "2023-02-28");

        let date = Date::from_ymd_opt(2023, 1, 1).unwrap();
        let prev = date.previous_day().unwrap();
        assert_eq!(prev.to_string(), "2022-12-31");

        // MIN_DATE has no previous day
        assert!(MIN_DATE.previous_day().is_none());
    }

    #[test]
    fn date_debug() {
        let date = Date::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(format!("{date:?}"), "Date(\"2023-06-15\")");
    }

    #[test]
    fn date_ordering() {
        let d1 = Date::from_ymd_opt(2023, 1, 1).unwrap();
        let d2 = Date::from_ymd_opt(2023, 1, 2).unwrap();
        let d3 = Date::from_ymd_opt(2023, 2, 1).unwrap();

        assert!(d1 < d2);
        assert!(d2 < d3);
        assert!(MIN_DATE < d1);

        // Equality
        let d1_copy = Date::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(d1, d1_copy);
    }

    #[test]
    fn date_display_padding() {
        let date = Date::from_ymd_opt(2023, 1, 5).unwrap();
        assert_eq!(date.to_string(), "2023-01-05");
        let date = Date::from_ymd_opt(123, 12, 31).unwrap();
        assert_eq!(date.to_string(), "0123-12-31");
    }
}
