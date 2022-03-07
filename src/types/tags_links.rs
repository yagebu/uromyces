use pyo3::types::PySet;
use pyo3::{IntoPy, PyObject, ToPyObject};
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
        TagsLinks(ThinVec::new())
    }

    /// Insert a tag or link. Returns whether it was newly inserted.
    pub fn insert(&mut self, value: String) -> bool {
        if self.0.iter().any(|v| *v == value) {
            false
        } else {
            self.0.push(value);
            true
        }
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

impl IntoPy<PyObject> for &TagsLinks {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        self.to_object(py)
    }
}

impl ToPyObject for TagsLinks {
    fn to_object(&self, py: pyo3::Python<'_>) -> PyObject {
        PySet::new(py, &self.0)
            .expect("creating a Python set to work")
            .into()
    }
}
