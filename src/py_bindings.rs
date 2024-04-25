//! Various helpers to interface with Python.
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::sync::GILOnceCell;
use pyo3::types::{PyAnyMethods, PyString};
use pyo3::PyTypeInfo;

use crate::types::{Amount, Date, Decimal};

/// Some Python classes/objects that we want to call/use from Rust.
pub struct PythonTypes {
    /// The `<AccountDummy>` object.
    pub account_dummy: PyObject,
    /// The `bool` object.
    pub bool: PyObject,
    /// The `datetime.date` object
    pub date: PyObject,
    /// The `decimal.Decimal` object
    pub decimal: PyObject,
    /// The `str` object
    pub str: PyObject,

    // The `uromyces.Amount` object
    // This is included since we want to use this type as a `dtype` for custom values.
    pub amount: PyObject,
}

static PY_TYPES: GILOnceCell<PythonTypes> = GILOnceCell::new();

/// Get the [`PythonTypes`] struct with some commonly use Python classes.
///
/// # Panics
///
/// Panics if the static has not been initialised (by a call to [`init_statics`]) yet.
pub fn get_python_types(py: Python) -> &PythonTypes {
    PY_TYPES.get(py).expect("static to be initialised")
}

/// Import decimal.Decimal and datetime.date
pub fn init_statics(py: Python) -> PyResult<()> {
    let builtins = py.import_bound("builtins")?;
    let bool = builtins.getattr("bool")?;
    let str = builtins.getattr("str")?;

    let decimal = py.import_bound("decimal")?.getattr("Decimal")?;
    let date = py.import_bound("datetime")?.getattr("date")?;

    PY_TYPES.get_or_init(py, || PythonTypes {
        account_dummy: PyString::new_bound(py, "<AccountDummy>").into(),
        bool: bool.into(),
        date: date.into(),
        decimal: decimal.into(),
        str: str.into(),

        amount: Amount::type_object_bound(py).into_any().into(),
    });

    Ok(())
}

/// Convert a [`Date`] to a Python `datetime.date`.
// pyo3 also provides this conversion but that uses the non-stable ABI
pub fn date_to_py(py: Python, date: Date) -> PyResult<PyObject> {
    get_python_types(py)
        .date
        .call1(py, (date.year(), date.month(), date.day()))
}

/// Convert a [`rust_decimal::Decimal`] to a Python decimal.Decimal.
// pyo3 also has this conversion but does a string conversion
pub fn decimal_to_py(py: Python, decimal: Decimal) -> PyObject {
    fn py_decimal_args(decimal: Decimal) -> (i32, Vec<u8>, i64) {
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
    }

    get_python_types(py)
        .decimal
        .call1(py, (py_decimal_args(decimal),))
        .expect("conversion to Python Decimal to succeed")
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
            assert_eq!(dec.to_string(), num);
        }

        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| -> PyResult<()> {
            init_statics(py).unwrap();

            roundtrip(py, "1.00");
            roundtrip(py, "1.0000000000000000000000000000");
            roundtrip(py, "1.1234567890123456789012345678");

            Ok(())
        })
        .unwrap();
    }
}
