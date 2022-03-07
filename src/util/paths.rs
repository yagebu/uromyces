use std::path::{Component, Path};

use glob;

use crate::types::FilePath;

#[derive(Debug)]
pub enum IncludeError {
    GlobReadError,
    InvalidBasePath,
    InvalidGlobPattern,
    NonUnicodePath,
}

/// For the given include directive, find matching files.
// TODO: consider restricting the allowed kinds of patterns.
pub fn glob_include(base_path: &FilePath, include: &str) -> Result<Vec<FilePath>, IncludeError> {
    let has_root = matches!(
        Path::new(include).components().next(),
        Some(Component::Prefix(..) | Component::RootDir)
    );

    let pattern = if has_root {
        include.to_string()
    } else {
        let dirname = base_path
            .as_ref()
            .parent()
            .ok_or(IncludeError::InvalidBasePath)?;
        dirname
            .join(include)
            .to_str()
            .ok_or(IncludeError::NonUnicodePath)?
            .to_string()
    };

    glob::glob(&pattern)
        .map_err(|_| IncludeError::InvalidGlobPattern)?
        .map(|glob_result| match glob_result {
            Err(_) => Err(IncludeError::GlobReadError),
            Ok(path) => match path.canonicalize() {
                Ok(p) => p
                    .as_path()
                    .try_into()
                    .map_err(|_| IncludeError::NonUnicodePath),
                Err(_) => Err(IncludeError::GlobReadError),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::FilePath;

    #[test]
    fn test_invalid_glob() {
        let path: FilePath = std::env::current_dir().unwrap().try_into().unwrap();
        assert!(glob_include(&path, "****").is_err());
    }

    #[test]
    fn test_glob() {
        let src_lib = std::env::current_dir().unwrap().join("src/lib.rs");
        let res = glob_include(&src_lib.try_into().unwrap(), "*.rs");
        assert!(res.is_ok());
        assert!(res.unwrap().len() > 6);
    }
}
