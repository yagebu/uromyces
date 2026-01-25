use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use serde::{Deserialize, Serialize};

use crate::types::Account;
use crate::types::interned_string::InternedString;

/// Type for file paths in uromyces.
///
/// Since we want to be able to freely print these filenames and use them in various contexts, we
/// only allow valid Unicode. Supporting non-Unicode paths in Beancount seems to be unnecessary.
/// This type can easily be created from `Path`s and `PathBuf`s that represent absolute and fully
/// Unicode paths via the `TryFrom` trait.
///
/// On creation `RealFilePath` ensures it always contains an absolute path. By using `.as_ref()` a
/// `Path` can be obtained to use all the standard path operations.
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize, IntoPyObjectRef)]
pub struct AbsoluteUTF8Path(InternedString);

/// Type for filenames in uromyces that might not be real paths.
///
/// This is either an absolute real file path (that is UTF-8) or a string of the form
/// `<summarize>`.
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize, IntoPyObjectRef)]
pub struct Filename(InternedString);

impl Filename {
    /// Internal helper to create `FilePath` from a path.
    fn from_ref(path: &str) -> Self {
        Self(path.into())
    }

    /// Create a dummy `Filename` - .
    #[must_use]
    pub fn new_dummy(dummy: &str) -> Self {
        let value = format!("<{dummy}>");
        Self(value.into())
    }
}

impl AbsoluteUTF8Path {
    /// Internal helper to create `RealFilePath` from a path that we know is absolute.
    fn from_ref(path: &str) -> Self {
        Self(path.into())
    }

    /// Converts to an owned `PathBuf`.
    fn to_path_buf(&self) -> PathBuf {
        Path::new(&*self.0).to_path_buf()
    }

    /// Join a path onto this one.
    pub(crate) fn join(&self, path: &str) -> Self {
        // self is absolute and Unicode-only, so the joined path is as well
        let joined = self.to_path_buf().join(path);
        Self::from_ref(joined.to_str().expect("valid UTF-8"))
    }

    /// Join an account onto this path.
    pub(crate) fn join_account(&self, account: &Account) -> Self {
        let mut joined = self.to_path_buf();
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
            dunce::canonicalize(&joined)
                .unwrap_or(joined)
                .to_str()
                .expect("valid UTF-8"),
        )
    }

    /// Create an absolute path from a string, resolving relative paths against a base file.
    ///
    /// If the path is already absolute, it is returned as-is. Otherwise, it is resolved
    /// relative to the parent directory of `base_file`.
    pub(crate) fn from_path_maybe_relative(
        path: &str,
        base_file: &Filename,
    ) -> Result<Self, FilePathError> {
        if Path::new(path).is_absolute() {
            Self::try_from(path)
        } else {
            let base = Self::try_from(base_file.clone())?;
            Ok(base.join_relative_to_file(path))
        }
    }
}

impl Deref for Filename {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for AbsoluteUTF8Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str: &str = &self.0;
        f.debug_tuple("AbsoluteUTF8Path").field(&str).finish()
    }
}
impl Debug for Filename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str: &str = &self.0;
        f.debug_tuple("Filename").field(&str).finish()
    }
}
impl Display for AbsoluteUTF8Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for Filename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<Path> for AbsoluteUTF8Path {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FilePathError {
    NonUnicode(PathBuf),
    NonAbsolute(String),
    NoRealFilePath(String),
}
impl std::error::Error for FilePathError {}
impl std::fmt::Display for FilePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::NonUnicode(p) => {
                write!(f, "Filepath is not valid unicode: {}", p.to_string_lossy())
            }
            Self::NonAbsolute(m) => write!(f, "Filepath is not absolute: '{m}'"),
            Self::NoRealFilePath(m) => write!(f, "String is no valid file path: '{m}'"),
        }
    }
}

impl From<AbsoluteUTF8Path> for Filename {
    fn from(value: AbsoluteUTF8Path) -> Self {
        Self(value.0)
    }
}
impl TryFrom<&str> for Filename {
    type Error = FilePathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with('<') {
            return Ok(Self::from_ref(value));
        }
        if !Path::new(value).is_absolute() {
            return Err(FilePathError::NonAbsolute(value.to_owned()));
        }
        Ok(Self::from_ref(value))
    }
}

impl TryFrom<&str> for AbsoluteUTF8Path {
    type Error = FilePathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !Path::new(value).is_absolute() {
            return Err(FilePathError::NonAbsolute(value.to_owned()));
        }
        Ok(Self::from_ref(value))
    }
}

impl TryFrom<Filename> for AbsoluteUTF8Path {
    type Error = FilePathError;

    fn try_from(value: Filename) -> Result<Self, Self::Error> {
        if value.starts_with('<') {
            return Err(FilePathError::NoRealFilePath(value.to_string()));
        }
        Ok(Self(value.0))
    }
}
impl TryFrom<&Path> for AbsoluteUTF8Path {
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
impl TryFrom<&Path> for Filename {
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

impl<'py> FromPyObject<'_, 'py> for AbsoluteUTF8Path {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = obj.extract::<PyBackedStr>()?;
        Ok(Self::try_from(&*str)?)
    }
}
impl<'py> FromPyObject<'_, 'py> for Filename {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = obj.extract::<PyBackedStr>()?;
        Ok(Self::try_from(&*str)?)
    }
}

impl From<FilePathError> for PyErr {
    fn from(value: FilePathError) -> Self {
        PyValueError::new_err(format!("Invalid filename: {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy() {
        assert_eq!(
            Filename::new_dummy("string"),
            Filename::try_from("<string>").unwrap()
        );
    }

    #[test]
    fn test_file_path_from() {
        assert!(Filename::try_from("asdf").is_err());
        assert!(Filename::try_from("<string>").is_ok());
    }

    #[test]
    fn test_real_file_path_from() {
        assert!(AbsoluteUTF8Path::try_from("asdf").is_err());
        assert!(AbsoluteUTF8Path::try_from(Path::new("asdf")).is_err());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_file_path_join_account() {
        let path = AbsoluteUTF8Path::try_from("/tmp/dir").unwrap();
        let account = "Assets:Cash".into();
        assert_eq!(
            path.join_account(&account),
            "/tmp/dir/Assets/Cash".try_into().unwrap()
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_from_path_maybe_relative_absolute() {
        let base = Filename::try_from("/base/dir/file.beancount").unwrap();
        let result = AbsoluteUTF8Path::from_path_maybe_relative("/absolute/path.pdf", &base);
        assert_eq!(result, AbsoluteUTF8Path::try_from("/absolute/path.pdf"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_from_path_maybe_relative_relative() {
        let base = Filename::try_from("/base/dir/file.beancount").unwrap();
        let result = AbsoluteUTF8Path::from_path_maybe_relative("relative/path.pdf", &base);
        assert_eq!(
            result,
            AbsoluteUTF8Path::try_from("/base/dir/relative/path.pdf")
        );
    }

    #[test]
    fn test_from_path_maybe_relative_dummy_filename() {
        let base = Filename::new_dummy("string");
        let result = AbsoluteUTF8Path::from_path_maybe_relative("relative/path.pdf", &base);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FilePathError::NoRealFilePath(_)
        ));
    }
}
