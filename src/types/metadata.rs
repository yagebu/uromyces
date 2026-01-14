use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyNone, PyType};
use pyo3::{BoundObject, IntoPyObjectExt};
use pyo3::{prelude::*, types::PyDict};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thin_vec::ThinVec;

use crate::types::{Account, Amount, Currency, Date, Decimal, Filename, LineNumber};

/// Possible metadata values (this is also used for custom entries).
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromPyObject, IntoPyObjectRef,
)]
#[serde(untagged)]
pub enum MetaValue {
    String(String),
    Account(Account),
    Tag(String),
    Date(Date),
    Bool(bool),
    Amount(Amount),
    Currency(Currency),
    Number(Decimal),
}

impl From<&str> for MetaValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

/// A single key-value pair in metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaKeyValuePair {
    pub key: String,
    pub value: Option<MetaValue>,
}

/// Metadata, a list of key-value pairs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Meta(ThinVec<MetaKeyValuePair>);

impl FromIterator<MetaKeyValuePair> for Meta {
    fn from_iter<T: IntoIterator<Item = MetaKeyValuePair>>(iter: T) -> Self {
        Meta(ThinVec::from_iter(iter))
    }
}

impl Meta {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn push(&mut self, value: MetaKeyValuePair) {
        self.0.push(value);
    }
    pub fn append(&mut self, other: &mut Meta) {
        self.0.append(&mut other.0);
    }
    pub fn remove(&mut self, key: &str) {
        if let Some(index) = self.0.iter().position(|v| v.key == key) {
            self.0.remove(index);
        }
    }
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.iter().any(|m| m.key == key)
    }

    /// Convert the metadata to a Python dict with the provied filename and lineno.
    ///
    /// # Errors
    ///
    /// Errors if a conversion to python or any of the `PyDict` operations fail.
    pub fn to_py_dict<'py>(
        &self,
        py: Python<'py>,
        filename: Option<&Filename>,
        line: Option<LineNumber>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let meta = PyDict::new(py);
        if let Some(filename) = filename {
            meta.set_item(pyo3::intern!(py, "filename"), filename)?;
        }
        if let Some(line) = line {
            meta.set_item(pyo3::intern!(py, "lineno"), line)?;
        }
        for kv in &self.0 {
            meta.set_item(&kv.key, &kv.value)?;
        }
        Ok(meta)
    }
}

/// The entry metadata which all entries carry.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[pyclass(frozen, mapping, module = "uromyces", skip_from_py_object)]
pub struct EntryMeta {
    /// Entry metadata.
    pub meta: Meta,
    /// The filename.
    #[pyo3(get)]
    pub filename: Filename,
    /// The 1-based line number.
    #[pyo3(get)]
    pub lineno: LineNumber,
}

impl Serialize for EntryMeta {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2 + self.meta.0.len()))?;
        map.serialize_entry("filename", &self.filename)?;
        map.serialize_entry("lineno", &self.lineno)?;
        for kv in &self.meta.0 {
            map.serialize_entry(&kv.key, &kv.value)?;
        }
        map.end()
    }
}

struct EntryMetaVisitor;

impl<'de> Visitor<'de> for EntryMetaVisitor {
    type Value = EntryMeta;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map with filename, lineno, and optional metadata keys")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut filename: Option<Filename> = None;
        let mut lineno: Option<LineNumber> = None;
        let mut meta = ThinVec::new();

        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                "filename" => {
                    filename = Some(access.next_value()?);
                }
                "lineno" => {
                    lineno = Some(access.next_value()?);
                }
                _ => {
                    meta.push(MetaKeyValuePair {
                        key,
                        value: access.next_value()?,
                    });
                }
            }
        }

        Ok(EntryMeta {
            filename: filename.ok_or_else(|| serde::de::Error::missing_field("filename"))?,
            lineno: lineno.ok_or_else(|| serde::de::Error::missing_field("lineno"))?,
            meta: Meta(meta),
        })
    }
}

impl<'de> Deserialize<'de> for EntryMeta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(EntryMetaVisitor)
    }
}

impl EntryMeta {
    /// Create a new entry header (with empty metadata).
    #[must_use]
    pub fn new(filename: Filename, lineno: LineNumber) -> Self {
        Self {
            meta: Meta::default(),
            filename,
            lineno,
        }
    }

    /// Create a new entry header (with empty metadata) from an existing one.
    #[must_use]
    pub fn from_existing(header: &Self) -> Self {
        Self::new(header.filename.clone(), header.lineno)
    }

    /// Add a metadata entry.
    pub fn add_meta(&mut self, key: &str, value: impl Into<MetaValue>) {
        self.meta.0.push(MetaKeyValuePair {
            key: key.to_owned(),
            value: Some(value.into()),
        });
    }

    pub(super) fn to_py_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.meta
            .to_py_dict(py, Some(&self.filename), Some(self.lineno))
    }
}

impl<'py> FromPyObject<'_, 'py> for EntryMeta {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(amount) = obj.cast::<Self>() {
            Ok(amount.get().clone())
        } else {
            let meta = obj.cast::<PyDict>()?;
            let (filename, line, meta) = extract_meta_dict(meta)?;
            Ok(Self {
                meta,
                filename: filename.ok_or_else(|| PyValueError::new_err("Missing filename"))?,
                lineno: line.ok_or_else(|| PyValueError::new_err("Missing lineno"))?,
            })
        }
    }
}

/// Extract metadata from Python dictionary.
fn extract_meta_dict(
    meta: Borrowed<'_, '_, PyDict>,
) -> PyResult<(Option<Filename>, Option<LineNumber>, Meta)> {
    let mut filename = None;
    let mut lineno = None;
    let meta_vec = meta
        .iter()
        .map(|(k, v)| {
            let key = k.extract::<String>()?;
            match key.as_str() {
                "filename" => {
                    filename = Some(v.extract::<Filename>()?);
                    Ok(None)
                }
                "lineno" => {
                    lineno = Some(v.extract::<u32>()?);
                    Ok(None)
                }
                _ => Ok(Some(MetaKeyValuePair {
                    key,
                    value: Some(MetaValue::extract(v.as_borrowed())?),
                })),
            }
        })
        .filter_map(Result::transpose)
        .collect::<PyResult<ThinVec<MetaKeyValuePair>>>()?;
    Ok((filename, lineno, Meta(meta_vec)))
}

#[pyclass]
struct EntryMetaKeysIter(std::vec::IntoIter<String>);

#[pymethods]
impl EntryMetaKeysIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<String> {
        slf.0.next()
    }
}

#[pymethods]
impl EntryMeta {
    #[new]
    fn __new__(meta: &Bound<'_, PyDict>) -> PyResult<Self> {
        let (filename, line, meta) = extract_meta_dict(meta.as_borrowed())?;
        Ok(Self {
            meta,
            filename: filename.ok_or_else(|| PyValueError::new_err("Missing filename"))?,
            lineno: line.ok_or_else(|| PyValueError::new_err("Missing lineno"))?,
        })
    }

    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let res = self.__getitem__(key, py);
        match res {
            Ok(e) => Ok(e),
            Err(error) if error.is_instance_of::<PyKeyError>(py) => {
                Ok(default.unwrap_or(PyNone::get(py).into_bound().into_any()))
            }
            Err(e) => Err(e),
        }
    }

    fn items<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static ITEMS_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        ITEMS_VIEW
            .import(py, "collections.abc", "ItemsView")?
            .call1((self.clone(),))
    }

    fn keys<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static KEYS_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        KEYS_VIEW
            .import(py, "collections.abc", "KeysView")?
            .call1((self.clone(),))
    }

    fn values<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static VALUES_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        VALUES_VIEW
            .import(py, "collections.abc", "ValuesView")?
            .call1((self.clone(),))
    }

    fn __len__(&self) -> usize {
        self.meta.0.len() + 2
    }

    fn __contains__(&self, key: &str) -> bool {
        match key {
            "filename" | "lineno" => true,
            _ => self.meta.0.iter().any(|m| m.key == key),
        }
    }

    fn __iter__(&self) -> EntryMetaKeysIter {
        let keys = ["filename".to_string(), "lineno".to_string()]
            .into_iter()
            .chain(self.meta.0.iter().map(|m| &m.key).cloned())
            .collect::<Vec<_>>();
        EntryMetaKeysIter(keys.into_iter())
    }

    fn __getitem__<'py>(&self, key: &str, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match key {
            "filename" => self.filename.into_bound_py_any(py),
            "lineno" => self.lineno.into_bound_py_any(py),
            _ => {
                let element = self.meta.0.iter().find(|m| m.key == key);
                match element {
                    Some(element) => element.value.into_bound_py_any(py),
                    None => Err(PyKeyError::new_err(key.to_owned())),
                }
            }
        }
    }
}

/// The posting metadata which postings carry.
/// Unlike `EntryMeta`, filename and line are optional since postings
/// may be generated by plugins without source locations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[pyclass(frozen, mapping, module = "uromyces", skip_from_py_object)]
pub struct PostingMeta {
    /// Posting metadata.
    pub meta: Meta,
    /// The optional filename.
    #[pyo3(get)]
    pub filename: Option<Filename>,
    /// The optional 1-based line number.
    #[pyo3(get)]
    pub lineno: Option<LineNumber>,
}

impl Serialize for PostingMeta {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let has_filename = self.filename.is_some();
        let has_line = self.lineno.is_some();
        let extra_fields = usize::from(has_filename) + usize::from(has_line);
        let mut map = serializer.serialize_map(Some(extra_fields + self.meta.0.len()))?;
        if let Some(ref filename) = self.filename {
            map.serialize_entry("filename", filename)?;
        }
        if let Some(line) = self.lineno {
            map.serialize_entry("lineno", &line)?;
        }
        for kv in &self.meta.0 {
            map.serialize_entry(&kv.key, &kv.value)?;
        }
        map.end()
    }
}

struct PostingMetaVisitor;

impl<'de> Visitor<'de> for PostingMetaVisitor {
    type Value = PostingMeta;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map with optional filename, lineno, and optional metadata keys")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut filename = None;
        let mut lineno = None;
        let mut meta = Meta::default();

        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                "filename" => {
                    filename = Some(access.next_value()?);
                }
                "lineno" => {
                    lineno = Some(access.next_value()?);
                }
                _ => {
                    meta.push(MetaKeyValuePair {
                        key,
                        value: access.next_value()?,
                    });
                }
            }
        }

        Ok(PostingMeta {
            meta,
            filename,
            lineno,
        })
    }
}

impl<'de> Deserialize<'de> for PostingMeta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(PostingMetaVisitor)
    }
}

impl PostingMeta {
    /// Create a new posting metadata with just a filename.
    #[must_use]
    pub(crate) fn with_filename(filename: Filename) -> Self {
        Self {
            meta: Meta::default(),
            filename: Some(filename),
            lineno: None,
        }
    }

    pub(crate) fn to_py_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.meta
            .to_py_dict(py, self.filename.as_ref(), self.lineno)
    }
}

impl<'py> FromPyObject<'_, 'py> for PostingMeta {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(meta) = obj.cast::<Self>() {
            Ok(meta.get().clone())
        } else {
            let meta = obj.cast::<PyDict>()?;
            let (filename, lineno, meta) = extract_meta_dict(meta)?;
            Ok(Self {
                meta,
                filename,
                lineno,
            })
        }
    }
}

#[pyclass]
struct PostingMetaKeysIter(std::vec::IntoIter<String>);

#[pymethods]
impl PostingMetaKeysIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<String> {
        slf.0.next()
    }
}

#[pymethods]
impl PostingMeta {
    #[new]
    fn __new__(meta: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let (filename, lineno, meta) = match meta {
            Some(meta) => extract_meta_dict(meta.as_borrowed())?,
            None => (None, None, Meta::default()),
        };
        Ok(Self {
            meta,
            filename,
            lineno,
        })
    }

    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let res = self.__getitem__(key, py);
        match res {
            Ok(e) => Ok(e),
            Err(error) if error.is_instance_of::<PyKeyError>(py) => {
                Ok(default.unwrap_or(PyNone::get(py).into_bound().into_any()))
            }
            Err(e) => Err(e),
        }
    }

    fn items<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static ITEMS_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        ITEMS_VIEW
            .import(py, "collections.abc", "ItemsView")?
            .call1((self.clone(),))
    }

    fn keys<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static KEYS_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        KEYS_VIEW
            .import(py, "collections.abc", "KeysView")?
            .call1((self.clone(),))
    }

    fn values<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        static VALUES_VIEW: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        VALUES_VIEW
            .import(py, "collections.abc", "ValuesView")?
            .call1((self.clone(),))
    }

    fn __len__(&self) -> usize {
        let extra = usize::from(self.filename.is_some()) + usize::from(self.lineno.is_some());
        self.meta.0.len() + extra
    }

    fn __contains__(&self, key: &str) -> bool {
        match key {
            "filename" => self.filename.is_some(),
            "lineno" => self.lineno.is_some(),
            _ => self.meta.0.iter().any(|m| m.key == key),
        }
    }

    fn __iter__(&self) -> PostingMetaKeysIter {
        let mut keys = Vec::new();
        if self.filename.is_some() {
            keys.push("filename".to_string());
        }
        if self.lineno.is_some() {
            keys.push("lineno".to_string());
        }
        keys.extend(self.meta.0.iter().map(|m| m.key.clone()));
        PostingMetaKeysIter(keys.into_iter())
    }

    fn __getitem__<'py>(&self, key: &str, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match key {
            "filename" => match &self.filename {
                Some(filename) => filename.into_bound_py_any(py),
                None => Err(PyKeyError::new_err(key.to_owned())),
            },
            "lineno" => match self.lineno {
                Some(line) => line.into_bound_py_any(py),
                None => Err(PyKeyError::new_err(key.to_owned())),
            },
            _ => {
                let element = self.meta.0.iter().find(|m| m.key == key);
                match element {
                    Some(element) => element.value.into_bound_py_any(py),
                    None => Err(PyKeyError::new_err(key.to_owned())),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_meta_serialize() {
        let meta = EntryMeta {
            filename: Filename::new_dummy("test"),
            lineno: 42,
            meta: Meta::default(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert_eq!(json, r#"{"filename":"<test>","lineno":42}"#);
    }

    #[test]
    fn test_entry_meta_serialize_with_meta() {
        let mut meta = EntryMeta {
            filename: Filename::new_dummy("test"),
            lineno: 42,
            meta: Meta::default(),
        };
        meta.add_meta("foo", "bar");
        let json = serde_json::to_string(&meta).unwrap();
        assert_eq!(json, r#"{"filename":"<test>","lineno":42,"foo":"bar"}"#);
    }

    #[test]
    fn test_entry_meta_deserialize() {
        let json = r#"{"filename":"<test>","lineno":42}"#;
        let meta: EntryMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.filename, Filename::new_dummy("test"));
        assert_eq!(meta.lineno, 42);
        assert!(meta.meta.is_empty());
    }

    #[test]
    fn test_entry_meta_roundtrip() {
        let mut original = EntryMeta {
            filename: Filename::new_dummy("example"),
            lineno: 100,
            meta: Meta::default(),
        };
        original.add_meta("note", "test note");

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: EntryMeta = serde_json::from_str(&json).unwrap();

        assert_eq!(original.filename, deserialized.filename);
        assert_eq!(original.lineno, deserialized.lineno);
    }

    #[test]
    fn test_posting_meta_serialize_empty() {
        let meta = PostingMeta::default();
        let json = serde_json::to_string(&meta).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_posting_meta_serialize_with_filename() {
        let meta = PostingMeta::with_filename(Filename::new_dummy("test"));
        let json = serde_json::to_string(&meta).unwrap();
        assert_eq!(json, r#"{"filename":"<test>"}"#);
    }

    #[test]
    fn test_posting_meta_deserialize_empty() {
        let json = "{}";
        let meta: PostingMeta = serde_json::from_str(json).unwrap();
        assert!(meta.filename.is_none());
        assert!(meta.lineno.is_none());
        assert!(meta.meta.is_empty());
    }

    #[test]
    fn test_posting_meta_deserialize_with_all() {
        let json = r#"{"filename":"<test>","lineno":42,"note":"hello"}"#;
        let meta: PostingMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.filename, Some(Filename::new_dummy("test")));
        assert_eq!(meta.lineno, Some(42));
        assert!(!meta.meta.is_empty());
    }
}
