//! Various helpers to interface with Python.
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyTuple, PyType};

pub use rust_decimal::Decimal;

/// Get the `decimal.Decimal` class from Python.
pub fn get_decimal_decimal(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
    static DECIMAL_DECIMAL: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    DECIMAL_DECIMAL.import(py, "decimal", "Decimal")
}

/// Convert a [`rust_decimal::Decimal`] to a Python decimal.Decimal.
// pyo3 also has this conversion but does a string conversion
pub fn decimal_to_py(py: Python<'_>, decimal: Decimal) -> PyResult<Bound<'_, PyAny>> {
    let (sign, digits, scale) = {
        let mut num = decimal.mantissa().abs();
        let mut digits: Vec<u8> = Vec::with_capacity(28);

        while num > 0 {
            #[allow(clippy::cast_sign_loss)]
            let n = (num % 10) as u8;
            num /= 10;
            digits.push(n);
        }
        digits.reverse();

        (
            i32::from(!decimal.is_sign_positive()),
            digits,
            -i64::from(decimal.scale()),
        )
    };
    let digits_tuple = PyTuple::new(py, digits)?;

    get_decimal_decimal(py)?.call1(((sign, digits_tuple, scale),))
}

/// Convert from a Python decimal.Decimal to a [`rust_decimal::Decimal`].
pub fn py_to_decimal(number: &Bound<'_, PyAny>) -> PyResult<Decimal> {
    let str = number.str()?.extract::<PyBackedStr>()?;
    Decimal::from_str(&str).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;

    use super::*;

    #[test]
    fn test_decimal_to_py() {
        fn roundtrip(py: Python, num: &str) {
            let d = Decimal::from_str(num).unwrap();
            let dec = decimal_to_py(py, d);
            assert_eq!(dec.unwrap().to_string(), num);
        }

        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            roundtrip(py, "1.00");
            roundtrip(py, "1.0000000000000000000000000000");
            roundtrip(py, "1.1234567890123456789012345678");

            Ok(())
        })
        .unwrap();
    }
}
