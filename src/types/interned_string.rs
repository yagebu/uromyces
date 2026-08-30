use std::borrow::Borrow;
use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use internment::ArcIntern;
use pyo3::sync::PyOnceLock;
use pyo3::{prelude::*, types::PyString};
use serde::{Deserialize, Serialize, Serializer};

/// The data in an [`InternedString`]: the string itself, plus a lazily-built Python `str` for it.
#[derive(Deserialize)]
#[serde(from = "String")]
struct InternedStringData {
    value: Box<str>,
    py_cache: PyOnceLock<Py<PyString>>,
}

impl PartialEq for InternedStringData {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for InternedStringData {}
impl PartialOrd for InternedStringData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for InternedStringData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
impl Hash for InternedStringData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
impl Borrow<str> for InternedStringData {
    fn borrow(&self) -> &str {
        &self.value
    }
}

impl From<&str> for InternedStringData {
    fn from(value: &str) -> Self {
        Self {
            value: value.into(),
            py_cache: PyOnceLock::new(),
        }
    }
}
impl From<String> for InternedStringData {
    fn from(value: String) -> Self {
        Self {
            value: value.into(),
            py_cache: PyOnceLock::new(),
        }
    }
}

impl Serialize for InternedStringData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

/// An interned string name.
///
/// This uses `ArcIntern` currently but is centralised here to allow for shared implementations of
/// the various traits.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InternedString(ArcIntern<InternedStringData>);

impl Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.value, f)
    }
}

impl Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0.value
    }
}

impl From<&str> for InternedString {
    fn from(value: &str) -> Self {
        Self(ArcIntern::from_ref(value))
    }
}

impl From<String> for InternedString {
    fn from(s: String) -> Self {
        Self(ArcIntern::new(s.into()))
    }
}

impl<'py> IntoPyObject<'py> for &InternedString {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let cached = self
            .0
            .py_cache
            .get_or_init(py, || PyString::new(py, &self.0.value).unbind());
        Ok(cached.bind(py).clone())
    }
}

impl<'py> FromPyObject<'_, 'py> for InternedString {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(obj.cast::<PyString>()?.to_str()?.into())
    }
}
