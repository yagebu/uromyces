use std::convert::Infallible;
use std::fmt::Display;
use std::hash::Hash;

use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::types::PyString;
use serde::{Deserialize, Serialize};

/// A wrapper around a `Box<str>`
///
/// We are dealing with immutable strings in most places, so avoid the memory of the capacity of a
/// `Vec`.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BoxStr(Box<str>);

impl Display for BoxStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for BoxStr {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for BoxStr {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl<'py> IntoPyObject<'py> for BoxStr {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0.into_pyobject(py)
    }
}

impl<'py> IntoPyObject<'py> for &BoxStr {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0.into_pyobject(py)
    }
}

impl<'py> FromPyObject<'_, 'py> for BoxStr {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = obj.extract::<PyBackedStr>()?;
        Ok((&*str).into())
    }
}
