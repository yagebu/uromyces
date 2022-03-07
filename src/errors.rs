//! Errors (which we might want to show to the user).
//!
//! There are many types of errors that reference some specific data in one of the
//! input files, so it might contain a filename and line number.
//! Otherwise, all information that should be displayed to the user about an error
//! should be contained in the error message.

use crate::types::{Entry, FilePath, LineNumber};

use serde::{Deserialize, Serialize};

/// This is a user-surfaceable error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UroError {
    /// The file that this error occured in (if it can be attributed).
    filename: Option<FilePath>,
    /// The line that this error occured on (if it can be attributed).
    line: Option<LineNumber>,
    /// The error message.
    message: String,
}

impl UroError {
    /// Get the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Create an error (without filename and line number).
    #[must_use]
    pub(crate) fn new<S: AsRef<str>>(message: S) -> Self {
        UroError {
            filename: None,
            line: None,
            message: message.as_ref().to_string(),
        }
    }

    /// Add a filename for the file that this error occurs in.
    #[must_use]
    pub(crate) fn with_filename(mut self, filename: &FilePath) -> Self {
        self.filename = Some(filename.clone());
        self
    }

    /// Add a position for the file and line that this error occurs in.
    #[must_use]
    pub(crate) fn with_position(mut self, filename: &Option<FilePath>, line: LineNumber) -> Self {
        self.filename = filename.clone();
        self.line = Some(line);
        self
    }

    /// Add a reference to the entry that this error occurs in.
    #[must_use]
    pub(crate) fn with_entry<E: Clone + Into<Entry>>(mut self, entry: &E) -> Self {
        let e: Entry = (*entry).clone().into();
        let header = e.get_header();
        self.filename = header.filename.clone();
        self.line = Some(header.line);
        self
    }
}
