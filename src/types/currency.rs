use std::fmt::{Debug, Display};

use internment::ArcIntern;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use serde::{Deserialize, Serialize};

/// A currency name.
///
/// This is a newtype wrapper so that we can transparently swap out the inner type
/// for a more fitting String-like type, make it immutable and avoid mixing them up with
/// other strings like account names.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Currency(ArcIntern<String>);

impl Debug for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Currency").field(&self.0.as_ref()).finish()
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.as_ref(), f)
    }
}

#[cfg(test)]
impl PartialEq<str> for Currency {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other
    }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self {
        Self(ArcIntern::from_ref(s))
    }
}

impl ToPyObject for Currency {
    fn to_object(&self, py: pyo3::Python<'_>) -> PyObject {
        self.0.to_object(py)
    }
}

impl IntoPy<PyObject> for Currency {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        self.0.to_object(py)
    }
}

impl<'py> FromPyObject<'py> for Currency {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let str = ob.extract::<PyBackedStr>()?;
        Ok(Self::from(&*str))
    }
}
