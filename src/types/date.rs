use std::fmt::Debug;

use chrono::{Datelike, NaiveDate};
use pyo3::{IntoPy, PyObject, ToPyObject};
use serde::{Deserialize, Serialize};

use crate::py_bindings::date_to_py;

/// A simple date.
///
/// Dates are currently stored as simple structs.
/// Once more date-related functionality is needed, `chrono::NaiveDate` should probably be used.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Date(NaiveDate);

impl Date {
    /// Try to parse a date from a string like "2012-12-12".
    pub(crate) fn try_from_str(s: &str) -> Result<Self, ()> {
        if s.len() < 10 {
            return Err(());
        }
        Ok(Date(
            NaiveDate::from_ymd_opt(
                s[0..4].parse().map_err(|_| ())?,
                s[5..7].parse().map_err(|_| ())?,
                s[8..10].parse().map_err(|_| ())?,
            )
            .ok_or(())?,
        ))
    }

    #[must_use]
    pub fn year(self) -> i32 {
        self.0.year()
    }

    #[must_use]
    pub fn month(self) -> u32 {
        self.0.month()
    }

    #[must_use]
    pub fn day(self) -> u32 {
        self.0.day()
    }
}

impl Debug for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date = format!("{:04}-{:02}-{:02}", self.year(), self.month(), self.day());
        f.debug_tuple("Date").field(&date).finish()
    }
}

impl ToPyObject for Date {
    fn to_object(&self, py: pyo3::Python<'_>) -> PyObject {
        date_to_py(py, *self).expect("creation of a datetime.date to work.")
    }
}

impl IntoPy<PyObject> for Date {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        self.to_object(py)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn date_from_str() {
        assert!(Date::try_from_str("2022").is_err());
        assert!(Date::try_from_str("2022-12-1").is_err());
        assert!(Date::try_from_str("2022-22-11").is_err());
        let date = Date::try_from_str("2022-12-12").unwrap();
        assert_eq!(date.year(), 2022);
    }

    #[test]
    fn date_serialisation() {
        let date = serde_json::from_str::<Date>("\"2022-12-12\"").unwrap();
        assert_eq!(serde_json::to_string(&date).unwrap(), "\"2022-12-12\"");
        assert!(serde_json::from_str::<Date>("\"2022\"").is_err());
        assert!(serde_json::from_str::<Date>("\"2022-12-111\"").is_err());
    }
}
