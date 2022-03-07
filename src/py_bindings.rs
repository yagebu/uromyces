use crate::types::{Date, Decimal};

use pyo3::once_cell::GILOnceCell;
use pyo3::prelude::{PyObject, PyResult, Python};

static PY_DATE: GILOnceCell<PyObject> = GILOnceCell::new();

static PY_DECIMAL: GILOnceCell<PyObject> = GILOnceCell::new();

/// Import decimal.Decimal and datetime.date
pub(crate) fn init_statics(py: Python) -> PyResult<()> {
    let decimal = py.import("decimal")?.getattr("Decimal")?;
    PY_DECIMAL.get_or_init(py, || decimal.into());

    let date = py.import("datetime")?.getattr("date")?;
    PY_DATE.get_or_init(py, || date.into());

    Ok(())
}

/// Convert a `[Date]` to a Python `datetime.date`.
// pyo3 also provides this conversion but that uses the non-stable ABI
pub(crate) fn date_to_py(py: Python, date: Date) -> PyResult<PyObject> {
    PY_DATE
        .get(py)
        .expect("static to be initialised")
        .call1(py, (date.year(), date.month(), date.day()))
}

/// Convert a `[rust_decimal::Decimal]` to a Python decimal.Decimal.
// pyo3 also has this conversion but does a string conversion
pub(crate) fn decimal_to_py(py: Python, decimal: Decimal) -> PyResult<PyObject> {
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

    PY_DECIMAL
        .get(py)
        .expect("static to be initialised")
        .call1(py, (py_decimal_args(decimal),))
}
