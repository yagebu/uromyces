//! Errors (which we might want to show to the user).
//!
//! There are many types of errors that reference some specific data in one of the
//! input files, so it might contain a filename and line number.
//! Otherwise, all information that should be displayed to the user about an error
//! should be contained in the error message.

use pyo3::{
    prelude::*,
    pybacked::PyBackedStr,
    types::{PyDict, PyMapping},
};
use serde::{Deserialize, Serialize};

use crate::types::{Entry, FilePath, LineNumber};

/// This is a user-surfaceable error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct UroError {
    /// The file that this error occured in (if it can be attributed).
    #[pyo3(get)]
    filename: Option<FilePath>,
    /// The line that this error occured on (if it can be attributed).
    #[pyo3(get)]
    line: Option<LineNumber>,
    /// The error message.
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl UroError {
    #[getter]
    /// Convert this to a Python dictionary like the `meta` attribute of Beancount entries.
    fn source<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let meta = PyDict::new_bound(py);
        meta.set_item(
            pyo3::intern!(py, "filename"),
            match &self.filename {
                Some(f) => f,
                None => "",
            },
        )?;
        meta.set_item(pyo3::intern!(py, "lineno"), self.line.unwrap_or(0))?;
        Ok(meta)
    }
}

/// Turn a Python object into a [`UroError`].
///
/// This expects the error object to look like the standard objects/namedtuples used for Beancount
/// errors. They should have a `.message` property containing a string and `.source` property
/// containing either `None` or a dict with a filename and line number.
///
/// The `entry` property that might also exist is currently ignored/discarded here.
pub(crate) fn error_from_py(error: &Bound<'_, PyAny>) -> PyResult<UroError> {
    let py = error.py();
    let msg = error
        .getattr(pyo3::intern!(py, "message"))?
        .extract::<String>()?;
    let error_source = error.getattr(pyo3::intern!(py, "source"))?;
    if error_source.is_none() {
        return Ok(UroError::new(msg));
    }
    let source = error_source.downcast::<PyMapping>()?;
    let filename = source
        .get_item(pyo3::intern!(py, "filename"))
        .ok()
        .and_then(|v| -> Option<PyBackedStr> { v.extract().ok()? })
        .and_then(|f| -> Option<FilePath> { (&*f).try_into().ok() });
    let lineno = source
        .get_item(pyo3::intern!(py, "lineno"))
        .ok()
        .and_then(|v| -> Option<LineNumber> { v.extract().ok()? });
    let uro_error = match (&filename, lineno) {
        (_, Some(l)) => UroError::new(msg).with_position(&filename, l),
        (Some(f), None) => UroError::new(msg).with_filename(f),
        (None, None) => UroError::new(msg),
    };
    Ok(uro_error)
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
            line: None,
            message: message.as_ref().to_string(),
        }
    }

    /// Add a filename for the file that this error occurs in.
    #[must_use]
    pub(crate) fn with_filename(mut self, filename: &FilePath) -> Self {
        self.filename = Some(filename.clone());
        self
    }

    /// Add a position for the file and line that this error occurs in.
    #[must_use]
    pub(crate) fn with_position(mut self, filename: &Option<FilePath>, line: LineNumber) -> Self {
        self.filename.clone_from(filename);
        self.line = Some(line);
        self
    }

    /// Add a reference to the entry that this error occurs in.
    #[must_use]
    pub(crate) fn with_entry<E: Clone + Into<Entry>>(mut self, entry: &E) -> Self {
        let e: Entry = (*entry).clone().into();
        let header = e.get_header();
        self.filename.clone_from(&header.filename);
        self.line = Some(header.line);
        self
    }
}
