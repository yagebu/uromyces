//! Errors (which we might want to show to the user).
//!
//! There are many types of errors that reference some specific data in one of the
//! input files, so it might contain a filename and line number.
//! Otherwise, all information that should be displayed to the user about an error
//! should be contained in the error message.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping};
use serde::{Deserialize, Serialize};

use crate::types::{Entry, Filename, LineNumber};

/// This is a user-surfaceable error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces", skip_from_py_object)]
pub struct UroError {
    /// The file that this error occured in (if it can be attributed).
    #[pyo3(get)]
    filename: Option<Filename>,
    /// The line that this error occured on (if it can be attributed).
    #[pyo3(get)]
    lineno: Option<LineNumber>,
    /// The error message.
    #[pyo3(get)]
    message: String,
    entry: Option<Box<Entry>>,
}

#[pymethods]
impl UroError {
    /// Convert this to a Python dictionary like the `meta` attribute of Beancount entries.
    #[getter]
    fn source<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let meta = PyDict::new(py);
        meta.set_item(
            pyo3::intern!(py, "filename"),
            match &self.filename {
                Some(f) => f,
                None => "",
            },
        )?;
        meta.set_item(pyo3::intern!(py, "lineno"), self.lineno.unwrap_or(0))?;
        Ok(meta)
    }
    #[getter]
    fn entry(&self) -> Option<Entry> {
        self.entry.as_ref().map(|b| *b.clone())
    }
}

// Turn a Python object into a [`UroError`].
//
// This expects the error object to look like the standard objects/namedtuples used for Beancount
// errors. They should have a `.message` property containing a string and `.source` property
// containing either `None` or a dict with a filename and line number.
//
// The `entry` property that might also exist is ignored/discarded here if it is not an uromyces
// Entry.
impl<'py> FromPyObject<'_, 'py> for UroError {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(uro_error) = obj.cast::<Self>() {
            Ok(uro_error.get().clone())
        } else {
            let py = obj.py();
            let mut error = UroError::new(
                obj.getattr(pyo3::intern!(py, "message"))?
                    .extract::<String>()?,
            );
            let source = obj.getattr(pyo3::intern!(py, "source"))?;
            if !source.is_none() {
                let source = source.cast::<PyMapping>()?;
                if let Ok(filename) = source.get_item(pyo3::intern!(py, "filename")) {
                    error.filename = Some(filename.extract()?);
                }
                if let Ok(line) = source.get_item(pyo3::intern!(py, "lineno")) {
                    error.lineno = Some(line.extract()?);
                }
            }
            let entry = obj.getattr(pyo3::intern!(py, "entry"))?;
            if !entry.is_none()
                && let Ok(entry) = entry.extract::<Entry>()
            {
                error.entry = Some(entry.into());
            }
            Ok(error)
        }
    }
}

impl UroError {
    /// Get the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Create an error (without filename and line number).
    #[must_use]
    pub(crate) fn new<S: AsRef<str>>(message: S) -> Self {
        Self {
            filename: None,
            lineno: None,
            message: message.as_ref().to_string(),
            entry: None,
        }
    }

    /// Add a filename for the file that this error occurs in.
    #[must_use]
    pub(crate) fn with_filename(mut self, filename: Filename) -> Self {
        self.filename = Some(filename);
        self
    }

    /// Add a position for the file and line that this error occurs in.
    #[must_use]
    pub(crate) fn with_position(mut self, filename: Filename, lineno: LineNumber) -> Self {
        self.filename = Some(filename);
        self.lineno = Some(lineno);
        self
    }

    /// Add a reference to the entry that this error occurs in.
    #[must_use]
    pub(crate) fn with_entry<E: Clone + Into<Entry>>(mut self, entry: &E) -> Self {
        let e: Entry = (*entry).clone().into();
        let header = e.meta();
        self.filename = Some(header.filename.clone());
        self.lineno = Some(header.lineno);
        self.entry = Some(e.into());
        self
    }
}
