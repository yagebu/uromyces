use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use internment::ArcIntern;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::types::PyString;
use serde::{Deserialize, Serialize};

use super::Account;

/// Type for filenames in uromyces.
///
/// Since we want to be able to freely print these filenames and use them in various contexts, we
/// only allow valid Unicode. Supporting non-Unicode paths in Beancount seems to be unnecessary.
/// This type can easily be created from `Path`s and `PathBuf`s that represent absolute and fully
/// Unicode paths via the `TryFrom` trait.
///
/// On creation `FilePath` ensures it always contains an absolute path. By using `.as_ref()` a
/// `Path` can be obtained to use all the standard path operations.
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePath(ArcIntern<String>);

impl FilePath {
    /// Internal helper to create `FilePath` from a path that we know is absolute.
    fn from_ref(path: &str) -> Self {
        Self(ArcIntern::from_ref(path))
    }

    /// Join a path onto this one.
    pub(crate) fn join(&self, path: &str) -> Self {
        // self is absolute and Unicode-only, so the joined path is as well
        let joined = self.as_ref().join(path);
        Self::from_ref(joined.to_str().expect("valid UTF-8"))
    }

    /// Join an account onto this path.
    pub(crate) fn join_account(&self, account: &Account) -> Self {
        let mut joined = self.as_ref().to_path_buf();
        joined.extend(account.components());
        // self is absolute and Unicode-only and so is the account, so the joined path is as well
        Self::from_ref(joined.to_str().expect("valid UTF-8"))
    }

    /// Join a path, relative to the parent dir of self.
    ///
    /// This also tries to canonicalize the path (but just emits the joined path on error of
    /// canonicalize).
    pub(crate) fn join_relative_to_file(&self, path: &str) -> Self {
        let dir = self.as_ref().parent().expect("path to have a parent");
        // self is absolute and Unicode-only, so the parent and joined path is as well
        let joined = dir.join(path);
        Self::from_ref(
            joined
                .canonicalize()
                .unwrap_or(joined)
                .to_str()
                .expect("valid UTF-8"),
        )
    }
}

impl Debug for FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FilePath").field(&self.0.as_ref()).finish()
    }
}

impl Deref for FilePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<Path> for FilePath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref().as_ref()
    }
}

#[derive(Debug)]
pub enum FilePathError {
    NonUnicode(PathBuf),
    NonAbsolute(String),
}
impl std::error::Error for FilePathError {}
impl std::fmt::Display for FilePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::NonUnicode(p) => {
                write!(f, "Filepath is not valid unicode: {}", p.to_string_lossy())
            }
            Self::NonAbsolute(m) => write!(f, "Filepath is not absolute: '{m}'"),
        }
    }
}

impl TryFrom<&str> for FilePath {
    type Error = FilePathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !Path::new(value).is_absolute() {
            return Err(FilePathError::NonAbsolute(value.to_owned()));
        }
        Ok(Self::from_ref(value))
    }
}

impl TryFrom<&Path> for FilePath {
    type Error = FilePathError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        match value.to_str() {
            Some(s) => {
                if value.is_absolute() {
                    Ok(Self::from_ref(s))
                } else {
                    Err(FilePathError::NonAbsolute(s.to_owned()))
                }
            }
            None => Err(FilePathError::NonUnicode(value.to_path_buf())),
        }
    }
}

impl TryFrom<PathBuf> for FilePath {
    type Error = FilePathError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(value.as_path())
    }
}

impl<'py> IntoPyObject<'py> for &FilePath {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0.as_str().into_pyobject(py)
    }
}

impl<'py> FromPyObject<'py> for FilePath {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let str = ob.extract::<PyBackedStr>()?;
        Ok(Self::try_from(&*str)?)
    }
}

impl From<FilePathError> for PyErr {
    fn from(_: FilePathError) -> Self {
        PyValueError::new_err("Invalid FilePath")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_from() {
        assert!(FilePath::try_from("asdf").is_err());
        assert!(FilePath::try_from(Path::new("asdf")).is_err());
    }

    #[test]
    fn test_file_path_join_account() {
        let path = FilePath::try_from("/tmp/dir").unwrap();
        let account = "Assets:Cash".into();
        assert_eq!(
            path.join_account(&account),
            "/tmp/dir/Assets/Cash".try_into().unwrap()
        );
    }
}
