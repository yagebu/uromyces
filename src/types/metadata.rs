use pyo3::exceptions::PyKeyError;
use pyo3::BoundObject;
use pyo3::{prelude::*, pybacked::PyBackedStr, types::PyDict};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thin_vec::ThinVec;

use crate::py_bindings::{decimal_to_py, py_to_decimal};

use super::{Account, Amount, Currency, Date, FilePath, LineNumber, TagsLinks};

/// Possible metadata values (this is also used for custom entries).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromPyObject)]
pub enum MetaValue {
    Account(Account),
    String(String),
    Tag(String),
    Date(Date),
    Bool(bool),
    Amount(Amount),
    Currency(Currency),
    Number(#[pyo3(from_py_with = "py_to_decimal")] Decimal),
}

impl From<&str> for MetaValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for MetaValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<'py> IntoPyObject<'py> for MetaValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        use MetaValue::{Account, Amount, Bool, Currency, Date, Number, String, Tag};
        match self {
            String(v) | Tag(v) => Ok(v.into_pyobject(py)?.into_any()),
            Date(v) => v.into_pyobject(py),
            Account(v) => Ok(v.into_pyobject(py)?.into_any()),
            Bool(v) => Ok(v.into_pyobject(py)?.into_any().to_owned()),
            Amount(v) => Ok(v.clone().into_pyobject(py)?.into_any()),
            Number(v) => decimal_to_py(py, v),
            Currency(v) => Ok(v.into_pyobject(py)?.into_any()),
        }
    }
}

impl<'a, 'py> IntoPyObject<'py> for &'a MetaValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        use MetaValue::{Account, Amount, Bool, Currency, Date, Number, String, Tag};
        match self {
            String(v) | Tag(v) => Ok(v.into_pyobject(py)?.into_any()),
            Date(v) => v.into_pyobject(py),
            Account(v) => Ok(v.into_pyobject(py)?.into_any()),
            Bool(v) => Ok(v.into_pyobject(py)?.into_any().to_owned()),
            Amount(v) => Ok(v.clone().into_pyobject(py)?.into_any()),
            Number(v) => decimal_to_py(py, *v),
            Currency(v) => Ok(v.into_pyobject(py)?.into_any()),
        }
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
        filename: &Option<FilePath>,
        line: LineNumber,
    ) -> PyResult<Bound<'py, PyDict>> {
        let meta = PyDict::new(py);
        meta.set_item(pyo3::intern!(py, "filename"), filename)?;
        meta.set_item(pyo3::intern!(py, "lineno"), line)?;
        for kv in &self.0 {
            meta.set_item(&kv.key, &kv.value)?;
        }
        Ok(meta)
    }
}

/// The "entry header", the data which all entries carry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct EntryHeader {
    /// Entry date.
    pub date: Date,
    /// Entry metadata.
    pub meta: Meta,
    /// Tags of the entry.
    #[pyo3(get)]
    pub tags: TagsLinks,
    /// Links of the entry.
    #[pyo3(get)]
    pub links: TagsLinks,
    /// The filename.
    #[pyo3(get)]
    pub filename: Option<FilePath>,
    /// The 1-based line number.
    #[pyo3(get)]
    pub line: LineNumber,
}

impl EntryHeader {
    /// Create a new entry header (with empty metadata, tags and links).
    #[must_use]
    pub fn new(date: Date, filename: Option<FilePath>, line: LineNumber) -> Self {
        Self {
            date,
            meta: Meta::default(),
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            filename,
            line,
        }
    }

    /// Create a new entry header (with empty metadata, tags and links) from an existing one.
    #[must_use]
    pub fn from_existing(header: &Self) -> Self {
        Self::new(header.date, header.filename.clone(), header.line)
    }

    /// Add a metadata entry.
    pub fn add_meta(&mut self, key: &str, value: impl Into<MetaValue>) {
        self.meta.0.push(MetaKeyValuePair {
            key: key.to_owned(),
            value: Some(value.into()),
        });
    }

    /// Convert this to a Python dictionary like the `meta` attribute of Beancount entries.
    pub(super) fn to_py_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.meta.to_py_dict(py, &self.filename, self.line)
    }

    /// Create a copy, possibly replacing the metadata, tags and links.
    pub(super) fn replace_meta_tags_links(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> PyResult<Self> {
        let (filename, line, meta) = match meta {
            Some(meta) => extract_meta_dict(meta)?,
            None => (self.filename.clone(), self.line, self.meta.clone()),
        };
        Ok(Self {
            date: date.unwrap_or(self.date),
            meta,
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            filename,
            line,
        })
    }
}

pub(super) fn extract_meta_dict(
    meta: &Bound<'_, PyDict>,
) -> PyResult<(Option<FilePath>, LineNumber, Meta)> {
    let mut filename = None;
    let mut line = 0;
    let meta_vec = meta
        .iter()
        .map(|(k, v)| {
            let key = k.extract::<String>()?;
            match &*key {
                "filename" => {
                    let filename_str = v.extract::<PyBackedStr>()?;
                    filename = (&*filename_str).try_into().ok();
                    Ok(None)
                }
                "lineno" => {
                    line = v.extract()?;
                    Ok(None)
                }
                _ => Ok(Some(MetaKeyValuePair {
                    key,
                    value: Some(MetaValue::extract_bound(&v)?),
                })),
            }
        })
        .filter_map(Result::transpose)
        .collect::<PyResult<ThinVec<MetaKeyValuePair>>>()?;
    Ok((filename, line, Meta(meta_vec)))
}

#[pymethods]
impl EntryHeader {
    #[new]
    #[pyo3(signature = (meta, date, tags=None, links=None))]
    fn __new__(
        meta: &Bound<'_, PyDict>,
        date: Date,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> PyResult<Self> {
        let (filename, line, meta) = extract_meta_dict(meta)?;
        Ok(Self {
            date,
            meta,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            filename,
            line,
        })
    }

    fn __len__(&self) -> usize {
        self.meta.0.len() + 2
    }

    fn __contains__(&self, key: &str) -> bool {
        match key {
            "lineno" | "filename" => true,
            _ => self.meta.0.iter().any(|m| m.key == key),
        }
    }

    fn __getitem__<'py>(&self, key: &str, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(match key {
            "filename" => self.filename.as_ref().into_pyobject(py)?,
            "lineno" => self.line.into_pyobject(py)?.into_any(),
            _ => {
                let element = self.meta.0.iter().find(|m| m.key == key);
                match element {
                    Some(element) => element.value.as_ref().into_pyobject(py)?,
                    None => Err(PyKeyError::new_err(""))?,
                }
            }
        })
    }
}
