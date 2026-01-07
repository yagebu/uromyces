//! Decimal numbers.
use std::fmt::Display;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyTuple, PyType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct DecimalError(rust_decimal::Error);

impl From<rust_decimal::Error> for DecimalError {
    fn from(from: rust_decimal::Error) -> Self {
        Self(from)
    }
}

impl std::error::Error for DecimalError {}

impl std::fmt::Display for DecimalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(
    Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Decimal(rust_decimal::Decimal);

impl Decimal {
    pub const ZERO: Decimal = Self(rust_decimal::Decimal::ZERO);
    pub const ONE: Decimal = Self(rust_decimal::Decimal::ONE);
    pub const TWO: Decimal = Self(rust_decimal::Decimal::TWO);

    /// Create a new decimal.
    #[must_use]
    pub fn new(num: i64, scale: u32) -> Self {
        Self(rust_decimal::Decimal::new(num, scale))
    }

    /// Test helper to create a Decimal from a string like `4.00`
    #[cfg(test)]
    pub(crate) fn d(s: &str) -> Self {
        Self::from_str_exact(s).expect("valid decimal in test")
    }

    /// Round to scale of twice the given tolerance.
    ///
    /// For midpoints, this rounds to the nearest even digit.
    #[must_use]
    pub(crate) fn round_with_tolerance(&self, tolerance: &Self) -> Self {
        let scale = (*tolerance * Decimal::TWO).0.normalize().scale();
        Self(self.0.round_dp(scale))
    }

    /// Check if the Decimal is zero.
    #[must_use]
    pub(crate) fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Check if sign of the Decimal is positive (also true for 0).
    #[must_use]
    pub(crate) fn is_sign_positive(&self) -> bool {
        self.0.is_sign_positive()
    }

    pub(crate) fn set_sign_positive(&mut self, positive: bool) {
        self.0.set_sign_positive(positive);
    }

    /// Get the absolute value of the Decimal.
    #[must_use]
    pub(crate) fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// Whether the two numbers have the same sign (or both are zero).
    #[must_use]
    pub(crate) fn eq_signum(&self, other: &Self) -> bool {
        if self.0.is_zero() || other.0.is_zero() {
            self.0.is_zero() && other.0.is_zero()
        } else {
            self.0.is_sign_negative() == other.0.is_sign_negative()
        }
    }

    /// Get the scale of the decimal number (which is 28 at max).
    #[must_use]
    pub(crate) fn scale(&self) -> u32 {
        self.0.scale()
    }

    /// Scale ONE to the scale of self or None if the scale of self is 0.
    #[must_use]
    pub(crate) fn scaled_one(&self) -> Option<Self> {
        let scale = self.0.scale();
        if scale > 0 {
            let mut scaled_one = Decimal::ONE;
            scaled_one
                .0
                .set_scale(scale)
                .expect("setting scale to scale of other Decimal to work");
            Some(scaled_one)
        } else {
            None
        }
    }

    /// Extract a Decimal from a string (ignoring commas).
    pub(crate) fn from_str_with_commas(s: &str) -> Result<Self, DecimalError> {
        if s.contains(',') {
            // FIXME(perf): this currently creates an intermediate String
            Self::from_str_exact(&s.replace(',', ""))
        } else {
            Self::from_str_exact(s)
        }
    }

    /// Extract a Decimal from a string.
    pub(crate) fn from_str_exact(s: &str) -> Result<Self, DecimalError> {
        Ok(rust_decimal::Decimal::from_str_exact(s).map(Self)?)
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(self.0.neg())
    }
}
impl Neg for &Decimal {
    type Output = Decimal;

    fn neg(self) -> Self::Output {
        Decimal(self.0.neg())
    }
}
impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.add(rhs.0))
    }
}
impl Add<&Decimal> for Decimal {
    type Output = Self;

    fn add(self, rhs: &Decimal) -> Self::Output {
        Self(self.0.add(rhs.0))
    }
}
impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        self.0.add_assign(rhs.0);
    }
}
impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.sub(rhs.0))
    }
}
impl SubAssign for Decimal {
    fn sub_assign(&mut self, rhs: Self) {
        self.0.sub_assign(rhs.0);
    }
}
impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.mul(rhs.0))
    }
}
impl Mul<&Decimal> for Decimal {
    type Output = Self;

    fn mul(self, rhs: &Decimal) -> Self::Output {
        Self(self.0.mul(rhs.0))
    }
}
impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0.div(rhs.0))
    }
}
impl Sum for Decimal {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}
impl<'a> Sum<&'a Decimal> for Decimal {
    fn sum<I: Iterator<Item = &'a Decimal>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

/// Get the `decimal.Decimal` class from Python.
pub fn get_decimal_decimal(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
    static DECIMAL_DECIMAL: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    DECIMAL_DECIMAL.import(py, "decimal", "Decimal")
}

impl<'a, 'py> FromPyObject<'a, 'py> for Decimal {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = &obj.str()?.extract::<PyBackedStr>()?;
        let dec = rust_decimal::Decimal::from_str(str).or_else(|_| {
            rust_decimal::Decimal::from_scientific(str)
                .map_err(|e| PyValueError::new_err(e.to_string()))
        })?;
        Ok(Decimal(dec))
    }
}

impl<'py> IntoPyObject<'py> for &Decimal {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let digits_tuple = {
            let mut num = self.0.mantissa().unsigned_abs();
            // The mantissa of a rust_decimal::Decimal is at most 96 bits and 28 < log_10(2^96) < 29
            let mut digits = [0u8; 29];
            let mut index = digits.len();
            while num > 0 {
                // This is fine since the remained will be 0 <= digit < 10
                #[allow(clippy::cast_sign_loss)]
                let digit = (num % 10) as u8;
                num /= 10;
                index -= 1;
                digits[index] = digit;
            }
            PyTuple::new(py, &digits[index..])?
        };

        get_decimal_decimal(py)?.call1(((
            i32::from(self.0.is_sign_negative()),
            digits_tuple,
            -i64::from(self.0.scale()),
        ),))
    }
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;

    use super::*;

    #[test]
    fn test_decimal_from_str() {
        assert_eq!(Decimal::from_str_exact("2.0000"), Ok(Decimal::new(2, 0)));
        assert_eq!(
            Decimal::from_str_exact("2.0000").unwrap().to_string(),
            "2.0000"
        );
        assert_eq!(Decimal::from_str_exact("1234"), Ok(Decimal::new(1234, 0)));
        assert_eq!(
            Decimal::from_str_exact("1_2_3_4"),
            Decimal::from_str_exact("1234")
        );
        let max_scale = Decimal::from_str_exact("0.0000000000000000000000000001");
        assert!(max_scale.is_ok());
        assert_eq!(max_scale.unwrap().scale(), 28);
        assert!(Decimal::from_str_exact("0.00000000000000000000000000001").is_err());

        assert!(Decimal::from_str_exact("1a11").is_err());
        assert!(Decimal::from_str_exact("++1").is_err());
        assert!(Decimal::from_str_exact("11111111111111111111111111111111111111111").is_err());
        assert!(Decimal::from_str_exact("0.000000000000000000000000000000000000001").is_err());
    }

    #[test]
    fn test_decimal_basics() {
        assert!(!Decimal::d("2.0000").is_zero());
        assert!(Decimal::d("0.0000").is_zero());
        assert!(Decimal::d("-0.0000").is_zero());
        assert!(!Decimal::d("-2.0000").is_zero());

        assert!(Decimal::d("2.0000").is_sign_positive());
        assert!(Decimal::d("0.0000").is_sign_positive());
        assert!(Decimal::d("-0.0000").is_sign_positive());
        assert!(Decimal::d("-0").is_sign_positive());
        assert!(!Decimal::d("-2.0000").is_sign_positive());
        // not exposed - it's exactly the opposite of is_sign_positive()
        assert!(!Decimal::d("2.0000").0.is_sign_negative());
        assert!(!Decimal::d("0.0000").0.is_sign_negative());
        assert!(!Decimal::d("-0.0000").0.is_sign_negative());
        assert!(Decimal::d("-2.0000").0.is_sign_negative());
    }

    #[test]
    fn test_decimal_scaled_one() {
        assert_eq!(Decimal::d("2").scaled_one(), None);
        assert_eq!(
            Decimal::d("2.0000").scaled_one(),
            Some(Decimal::d("0.0001"))
        );
        assert_eq!(
            Decimal::d("2.0001").scaled_one(),
            Some(Decimal::d("0.0001"))
        );
        assert_eq!(Decimal::d("2.01").scaled_one(), Some(Decimal::d("0.01")));
    }

    #[test]
    fn test_decimal_round_with_tolerance() {
        let tol = Decimal::d("0.05");
        assert_eq!(
            Decimal::d("1.2345").round_with_tolerance(&tol),
            Decimal::d("1.2")
        );
        let tol = Decimal::d("0.005");
        assert_eq!(
            Decimal::d("1.2345").round_with_tolerance(&tol),
            Decimal::d("1.23")
        );
        assert_eq!(
            Decimal::d("1.235").round_with_tolerance(&tol),
            Decimal::d("1.24")
        );
        assert_eq!(
            Decimal::d("1.245").round_with_tolerance(&tol),
            Decimal::d("1.24")
        );
    }

    #[test]
    fn test_decimal_same_signum() {
        assert!(Decimal::d("2").eq_signum(&Decimal::d("1")));
        assert!(Decimal::d("-2").eq_signum(&Decimal::d("-1")));
        assert!(Decimal::d("0").eq_signum(&Decimal::d("0")));
        assert!(Decimal::d("0").eq_signum(&Decimal::d("-0")));

        assert!(!Decimal::d("2").eq_signum(&Decimal::d("0")));
        assert!(!Decimal::d("2").eq_signum(&Decimal::d("-2")));
        assert!(!Decimal::d("-2").eq_signum(&Decimal::d("0")));
        assert!(!Decimal::d("0").eq_signum(&Decimal::d("-2")));
    }

    #[test]
    fn test_decimal_to_py() {
        fn roundtrip(py: Python, num: &str) {
            let d = Decimal::d(num);
            let dec = d.into_pyobject(py);
            assert_eq!(dec.unwrap().to_string(), num);
        }

        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            roundtrip(py, "0");
            roundtrip(py, "1.00");
            roundtrip(py, "-1.23456");
            roundtrip(py, "1.23456");
            roundtrip(py, "1.23000");
            roundtrip(py, "1.0000000000000000000000000000");
            roundtrip(py, "1.1234567890123456789012345678");
            roundtrip(py, &Decimal(rust_decimal::Decimal::MAX).to_string());
            roundtrip(py, &Decimal(rust_decimal::Decimal::MIN).to_string());

            Ok(())
        })
        .unwrap();
    }
}
