use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors::UroError;
use crate::options::BeancountOptions;
#[cfg(test)]
use crate::parse::ParsedFile;
use crate::types::{Entry, FilePath, Plugin, RawEntry};

/// The result of parsing a Beancount file and all its includes.
#[derive(Debug)]
pub(crate) struct RawLedger {
    /// The main filename.
    pub filename: FilePath,
    /// The (raw) sorted entries of the ledger.
    pub entries: Vec<RawEntry>,
    /// Errors encountered on converting the parse tree to a ParseResult.
    pub errors: Vec<UroError>,
    /// The options in the file.
    pub options: BeancountOptions,
    // Included file paths.
    pub includes: Vec<FilePath>,
    /// Plugins (with optional config)
    pub plugins: Vec<Plugin>,
}

impl RawLedger {
    /// New raw ledger for a given file, with the given includes and the expected entry count.
    pub(crate) fn from_filename_and_includes(
        filename: FilePath,
        includes: Vec<FilePath>,
        entry_count: usize,
    ) -> Self {
        Self {
            filename,
            entries: Vec::with_capacity(entry_count),
            errors: Vec::default(),
            options: BeancountOptions::default(),
            includes,
            plugins: Vec::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_single_parsed_file(filename: FilePath, parsed_file: ParsedFile) -> Self {
        Self {
            filename,
            entries: parsed_file.entries,
            errors: parsed_file.errors,
            options: BeancountOptions::default(),
            includes: Vec::new(),
            plugins: Vec::new(),
        }
    }
}

/// The result of parsing a Beancount file and all its includes and running booking.
#[derive(Debug, Serialize, Deserialize)]
#[pyclass]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct Ledger {
    /// The main filename.
    pub filename: FilePath,
    /// The entries of the ledger (sorted).
    #[pyo3(get)]
    pub entries: Vec<Entry>,
    /// Errors that occured on parsing, booking or any later stage.
    pub errors: Vec<UroError>,
    /// The options in the file.
    pub options: BeancountOptions,
    // Included file paths.
    pub includes: Vec<FilePath>,
    /// Plugins (with optional config)
    pub plugins: Vec<Plugin>,
}

impl Ledger {
    /// Create a ledger from the underlying raw ledger with an empty list of entries.
    pub(crate) fn from_raw_empty_entries(raw_ledger: &RawLedger) -> Self {
        Self {
            filename: raw_ledger.filename.clone(),
            entries: Vec::new(),
            errors: raw_ledger.errors.clone(),
            options: raw_ledger.options.clone(),
            includes: raw_ledger.includes.clone(),
            plugins: raw_ledger.plugins.clone(),
        }
    }
}
