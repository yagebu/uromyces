use pyo3::prelude::*;
use pyo3::types::{PyFrozenSet, PySet};
use serde::{Deserialize, Serialize};
use thin_vec::ThinVec;

/// A set of tags or a set of links.
///
/// We want this to be a set (i.e. contain no duplicate elements) and preserve insertion order.
/// Since these sets tend to be small, we can get away with having a bare Vec as a backing storage.
/// A more performant solution could be something like the indexmap crate (the Rust standard-library
/// `HashSet` does not preserve insertion order).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagsLinks(ThinVec<String>);

impl TagsLinks {
    #[must_use]
    pub fn new() -> Self {
        Self(ThinVec::new())
    }

    /// Insert a tag or link. Returns whether it was newly inserted.
    pub fn insert(&mut self, value: String) -> bool {
        if self.0.contains(&value) {
            false
        } else {
            self.0.push(value);
            true
        }
    }

    /// Check whether a certain value is contained in the set.
    #[must_use]
    pub fn contains(&self, value: &str) -> bool {
        self.0.iter().any(|v| *v == value)
    }

    /// Reomve a tag or link. Returns whether it was present in the set.
    pub fn remove(&mut self, value: &str) -> bool {
        if let Some(index) = self.0.iter().position(|v| *v == value) {
            self.0.remove(index);
            true
        } else {
            false
        }
    }
}

impl Default for TagsLinks {
    fn default() -> Self {
        Self::new()
    }
}

impl<'py> IntoPyObject<'py> for &TagsLinks {
    type Target = PyFrozenSet;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        PyFrozenSet::new(py, &self.0)
    }
}

impl<'py> FromPyObject<'_, 'py> for TagsLinks {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let vec = if let Ok(set) = obj.cast::<PySet>() {
            set.iter()
                .filter_map(|e| e.extract::<String>().ok())
                .collect()
        } else {
            let set = obj.cast::<PyFrozenSet>()?;
            set.iter()
                .filter_map(|e| e.extract::<String>().ok())
                .collect()
        };
        Ok(Self(vec))
    }
}
