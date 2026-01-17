//! Conversion of uromyces entries to beancount.core.data namedtuples.
use pyo3::IntoPyObjectExt;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyNone;
use pyo3::{prelude::*, types::PyAnyMethods, types::PyType};

use crate::types::{
    Amount, Balance, Close, Commodity, Cost, Custom, CustomValue, Document, Event, Note, Open, Pad,
    Posting, Price, Query, Transaction,
};

pub(super) trait ConvertToBeancount {
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
        static AMOUNT: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        AMOUNT
            .import(py, "beancount.core.amount", "Amount")?
            .call1((&self.number, &self.currency))
    }
}

impl ConvertToBeancount for Cost {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static AMOUNT: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        AMOUNT
            .import(py, "beancount.core.position", "Cost")?
            .call1((&self.number, &self.currency, &self.date, &self.label))
    }
}

impl ConvertToBeancount for CustomValue {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static VALUE_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        VALUE_TYPE
            .import(py, "beancount.parser.grammar", "ValueType")?
            .call1((&self.0, self.dtype(py)?))
    }
}

impl ConvertToBeancount for Posting {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static POSTING: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        POSTING
            .import(py, "beancount.core.data", "Posting")?
            .call1((
                &self.account,
                &self.units.convert_to_beancount(py)?,
                &self.cost.convert_to_beancount(py)?,
                &self.price.convert_to_beancount(py)?,
                &self.flag,
                self.meta.copy(py)?,
            ))
    }
}

impl ConvertToBeancount for Balance {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static BALANCE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        BALANCE
            .import(py, "beancount.core.data", "Balance")?
            .call1((
                self.meta.copy(py)?,
                &self.date,
                &self.account,
                &self.amount.convert_to_beancount(py)?,
                &self.tolerance,
                PyNone::get(py),
            ))
    }
}

impl ConvertToBeancount for Commodity {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static COMMODITY: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        COMMODITY
            .import(py, "beancount.core.data", "Commodity")?
            .call1((self.meta.copy(py)?, &self.date, &self.currency))
    }
}

impl ConvertToBeancount for Close {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static CLOSE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        CLOSE.import(py, "beancount.core.data", "Close")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
        ))
    }
}

impl ConvertToBeancount for Custom {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static CUSTOM: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        CUSTOM.import(py, "beancount.core.data", "Custom")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.r#type,
            self.values.convert_to_beancount(py)?,
        ))
    }
}

impl ConvertToBeancount for Document {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static DOCUMENT: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        DOCUMENT
            .import(py, "beancount.core.data", "Document")?
            .call1((
                self.meta.copy(py)?,
                &self.date,
                &self.account,
                &self.filename,
                &self.tags,
                &self.links,
            ))
    }
}

impl ConvertToBeancount for Event {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static EVENT: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        EVENT.import(py, "beancount.core.data", "Event")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.r#type,
            &self.description,
        ))
    }
}

impl ConvertToBeancount for Note {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static NOTE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        NOTE.import(py, "beancount.core.data", "Note")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.comment,
            &self.tags,
            &self.links,
        ))
    }
}

impl ConvertToBeancount for Open {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static BOOKING: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        static OPEN: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        let booking = BOOKING.import(py, "beancount.core.data", "Booking")?;
        OPEN.import(py, "beancount.core.data", "Open")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            if self.currencies.is_empty() {
                PyNone::get(py).into_bound_py_any(py)?
            } else {
                self.currencies.into_bound_py_any(py)?
            },
            match self.booking {
                Some(b) => booking.getattr(b.value(py))?,
                None => PyNone::get(py).into_bound_py_any(py)?,
            },
        ))
    }
}

impl ConvertToBeancount for Pad {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static PAD: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        PAD.import(py, "beancount.core.data", "Pad")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.account,
            &self.source_account,
        ))
    }
}

impl ConvertToBeancount for Price {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static PRICE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        PRICE.import(py, "beancount.core.data", "Price")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.currency,
            self.amount.convert_to_beancount(py)?,
        ))
    }
}

impl ConvertToBeancount for Transaction {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static TRANSACTION: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        TRANSACTION
            .import(py, "beancount.core.data", "Transaction")?
            .call1((
                self.meta.copy(py)?,
                &self.date,
                &self.flag,
                &self.payee,
                &self.narration,
                &self.tags,
                &self.links,
                self.postings.convert_to_beancount(py)?,
            ))
    }
}
impl ConvertToBeancount for Query {
    fn convert_to_beancount<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static QUERY: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        QUERY.import(py, "beancount.core.data", "Query")?.call1((
            self.meta.copy(py)?,
            &self.date,
            &self.name,
            &self.query_string,
        ))
    }
}
