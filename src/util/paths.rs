use std::path::{Component, Path};

use glob;

use crate::types::AbsoluteUTF8Path;

/// An error that might be encountered on reading a glob.
#[derive(Debug)]
pub enum GlobIncludeError {
    BasePathHasNoParent,
    GlobReadError,
    InvalidGlobPattern(String),
    NonUnicodePath,
}

impl std::error::Error for GlobIncludeError {}
impl std::fmt::Display for GlobIncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::BasePathHasNoParent => {
                write!(f, "base path has not parent folder")
            }
            Self::GlobReadError => {
                write!(f, "IO error on reading glob")
            }
            Self::InvalidGlobPattern(msg) => {
                write!(f, "Invalid glob pattern: {msg}")
            }
            Self::NonUnicodePath => {
                write!(f, "encountered non-Unicode path during glob")
            }
        }
    }
}

/// For the given include directive, find matching files.
// TODO: consider restricting the allowed kinds of patterns.
pub fn glob_include(
    base_path: &AbsoluteUTF8Path,
    include: &str,
) -> Result<Vec<AbsoluteUTF8Path>, GlobIncludeError> {
    let has_root = matches!(
        Path::new(include).components().next(),
        Some(Component::Prefix(..) | Component::RootDir)
    );

    let pattern = if has_root {
        include.to_owned()
    } else {
        let dirname = base_path
            .as_ref()
            .parent()
            .ok_or(GlobIncludeError::BasePathHasNoParent)?;
        dirname
            .join(include)
            .to_str()
            .expect("paths joined from unicode parts to be unicode")
            .to_owned()
    };

    glob::glob(&pattern)
        .map_err(|e| GlobIncludeError::InvalidGlobPattern(e.msg.to_owned()))?
        .map(|glob_result| match glob_result {
            Err(_) => Err(GlobIncludeError::GlobReadError),
            Ok(path) => match path.canonicalize() {
                Ok(p) => p
                    .as_path()
                    .try_into()
                    .map_err(|_| GlobIncludeError::NonUnicodePath),
                Err(_) => Err(GlobIncludeError::GlobReadError),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::AbsoluteUTF8Path;

    #[test]
    fn test_invalid_glob() {
        let path: AbsoluteUTF8Path = std::env::current_dir()
            .unwrap()
            .as_path()
            .try_into()
            .unwrap();
        let err = glob_include(&path, "****").unwrap_err();
        let GlobIncludeError::InvalidGlobPattern(msg) = err else {
            panic!();
        };
        assert!(msg.contains("wildcards"));
    }

    #[test]
    fn test_glob() {
        let src_lib = std::env::current_dir().unwrap().join("src/lib.rs");
        let res = glob_include(&src_lib.as_path().try_into().unwrap(), "*.rs");
        assert!(res.is_ok());
        assert!(res.unwrap().len() > 6);
    }
}
