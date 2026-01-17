use std::convert::Infallible;
use std::fmt::Display;
use std::ops::Deref;

use internment::ArcIntern;
use pyo3::{prelude::*, pybacked::PyBackedStr, types::PyString};
use serde::{Deserialize, Serialize};

/// An interned string name.
///
/// This uses `ArcIntern` currently but is centralised here to allow for shared implementations of
/// the various traits.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InternedString(ArcIntern<String>);

impl Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<&str> for InternedString {
    fn from(s: &str) -> Self {
        Self(ArcIntern::from_ref(s))
    }
}

impl From<String> for InternedString {
    fn from(s: String) -> Self {
        Self(ArcIntern::from(s))
    }
}

impl<'py> IntoPyObject<'py> for &InternedString {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0.as_str().into_pyobject(py)
    }
}

impl<'py> FromPyObject<'_, 'py> for InternedString {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = obj.extract::<PyBackedStr>()?;
        Ok((&*str).into())
    }
}
