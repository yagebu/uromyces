//! Conversion of uromyces entries to beancount.core.data namedtuples.
use pyo3::IntoPyObjectExt;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyNone;
use pyo3::{prelude::*, types::PyAnyMethods, types::PyType};

use crate::types::{
    Amount, Balance, Booking, Close, Commodity, Cost, Custom, CustomValue, Document, Entry, Event,
    Note, Open, Pad, Posting, Price, Query, Transaction,
};

/// Get the `NamedTuple._make` classmethod of the given class, bound and cached in `cache`
/// so that the module import, attribute lookups, and classmethod binding only happen once.
///
/// Calling the returned classmethod (with a single tuple of fields, e.g. `make.call1((fields,))`)
/// constructs the `NamedTuple` directly, bypassing the field-by-field `__new__` (positional-
/// argument binding, plus any assertions such as `Amount.__new__`'s `isinstance` checks). This is
/// safe as long as the fields passed are of the correct type for the target class, which we
/// always do here.
fn get_make<'py>(
    py: Python<'py>,
    cache: &'static PyOnceLock<Py<PyAny>>,
    module_name: &str,
    class_name: &str,
) -> PyResult<&'py Bound<'py, PyAny>> {
    cache
        .get_or_try_init(py, || -> PyResult<Py<PyAny>> {
            let make = py
                .import(module_name)?
                .getattr(class_name)?
                .getattr("_make")?;
            Ok(make.unbind())
        })
        .map(|make| make.bind(py))
}

pub(crate) trait ConvertToBeancount {
    /// Convert an object to its matching Beancount type.
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
}

impl<T: ConvertToBeancount> ConvertToBeancount for Option<T> {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Some(s) => s.convert_to_beancount(py),
            None => PyNone::get(py).into_bound_py_any(py),
        }
    }
}

impl<T: ConvertToBeancount> ConvertToBeancount for Vec<T> {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.iter()
            .map(|v| v.convert_to_beancount(py))
            .collect::<PyResult<Vec<_>>>()?
            .into_bound_py_any(py)
    }
}

impl ConvertToBeancount for Amount {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static AMOUNT: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &AMOUNT, "beancount.core.amount", "Amount")?
            .call1(((&self.number, &self.currency),))
    }
}

impl ConvertToBeancount for Cost {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static COST: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &COST, "beancount.core.position", "Cost")?.call1(((
            &self.number,
            &self.currency,
            &self.date,
            &self.label,
        ),))
    }
}

impl ConvertToBeancount for CustomValue {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static VALUE_TYPE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &VALUE_TYPE, "beancount.parser.grammar", "ValueType")?
            .call1(((&self.0, self.dtype(py)?),))
    }
}

impl ConvertToBeancount for Posting {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static POSTING: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &POSTING, "beancount.core.data", "Posting")?.call1(((
            &self.account,
            &self.units.convert_to_beancount(py)?,
            &self.cost.convert_to_beancount(py)?,
            &self.price.convert_to_beancount(py)?,
            &self.flag,
            self.meta.copy(py)?,
        ),))
    }
}

impl ConvertToBeancount for Balance {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static BALANCE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &BALANCE, "beancount.core.data", "Balance")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.amount.convert_to_beancount(py)?,
            &self.tolerance,
            PyNone::get(py),
        ),))
    }
}

impl ConvertToBeancount for Commodity {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static COMMODITY: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &COMMODITY, "beancount.core.data", "Commodity")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.currency,
        ),))
    }
}

impl ConvertToBeancount for Close {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static CLOSE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &CLOSE, "beancount.core.data", "Close")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
        ),))
    }
}

impl ConvertToBeancount for Custom {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static CUSTOM: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &CUSTOM, "beancount.core.data", "Custom")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.r#type,
            self.values.convert_to_beancount(py)?,
        ),))
    }
}

impl ConvertToBeancount for Document {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static DOCUMENT: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &DOCUMENT, "beancount.core.data", "Document")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.filename,
            &self.tags,
            &self.links,
        ),))
    }
}

impl ConvertToBeancount for Event {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static EVENT: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &EVENT, "beancount.core.data", "Event")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.r#type,
            &self.description,
        ),))
    }
}

impl ConvertToBeancount for Note {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static NOTE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &NOTE, "beancount.core.data", "Note")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.comment,
            &self.tags,
            &self.links,
        ),))
    }
}

impl ConvertToBeancount for Booking {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static BOOKING: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        let booking = BOOKING.import(py, "beancount.core.data", "Booking")?;
        booking.getattr(self.value(py))
    }
}

impl ConvertToBeancount for Open {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static OPEN: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &OPEN, "beancount.core.data", "Open")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            if self.currencies.is_empty() {
                PyNone::get(py).into_bound_py_any(py)?
            } else {
                self.currencies.into_bound_py_any(py)?
            },
            self.booking.convert_to_beancount(py)?,
        ),))
    }
}

impl ConvertToBeancount for Pad {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static PAD: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &PAD, "beancount.core.data", "Pad")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.source_account,
        ),))
    }
}

impl ConvertToBeancount for Price {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static PRICE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &PRICE, "beancount.core.data", "Price")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.currency,
            self.amount.convert_to_beancount(py)?,
        ),))
    }
}

impl ConvertToBeancount for Transaction {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static TRANSACTION: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &TRANSACTION, "beancount.core.data", "Transaction")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.flag,
            &self.payee,
            &self.narration,
            &self.tags,
            &self.links,
            self.postings.convert_to_beancount(py)?,
        ),))
    }
}
impl ConvertToBeancount for Query {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static QUERY: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        get_make(py, &QUERY, "beancount.core.data", "Query")?.call1(((
            self.meta.copy(py)?,
            &self.date,
            &self.name,
            &self.query_string,
        ),))
    }
}

impl ConvertToBeancount for Entry {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Balance(e) => e.convert_to_beancount(py),
            Self::Close(e) => e.convert_to_beancount(py),
            Self::Commodity(e) => e.convert_to_beancount(py),
            Self::Custom(e) => e.convert_to_beancount(py),
            Self::Document(e) => e.convert_to_beancount(py),
            Self::Event(e) => e.convert_to_beancount(py),
            Self::Note(e) => e.convert_to_beancount(py),
            Self::Open(e) => e.convert_to_beancount(py),
            Self::Pad(e) => e.convert_to_beancount(py),
            Self::Price(e) => e.convert_to_beancount(py),
            Self::Query(e) => e.convert_to_beancount(py),
            Self::Transaction(e) => e.convert_to_beancount(py),
        }
    }
}
