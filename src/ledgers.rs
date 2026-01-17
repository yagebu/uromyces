//!  Ledgers encompass all the data from parsed and booked input Beancount journals.
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors::UroError;
use crate::options::BeancountOptions;
#[cfg(test)]
use crate::parse::ParsedFile;
use crate::plugins::{run_named_plugin, run_validations};
use crate::types::{Entry, Filename, Plugin, RawEntry};

/// The result of parsing a Beancount file and all its includes.
#[derive(Debug, Clone)]
pub struct RawLedger {
    /// The main filename.
    pub filename: Filename,
    /// The (raw) sorted entries of the ledger.
    pub entries: Vec<RawEntry>,
    /// Errors encountered on converting the parse tree to a `ParseResult`.
    pub errors: Vec<UroError>,
    /// The options in the file.
    pub options: BeancountOptions,
    /// Included file paths.
    pub includes: Vec<Filename>,
    /// Plugins (with optional config)
    pub plugins: Vec<Plugin>,
}

impl RawLedger {
    /// New raw ledger for a given file, with the given includes and the expected entry count.
    pub(crate) fn from_filename_and_includes(
        filename: Filename,
        includes: Vec<Filename>,
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
    pub(crate) fn from_single_parsed_file(filename: Filename, parsed_file: ParsedFile) -> Self {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[pyclass(module = "uromyces")]
pub struct Ledger {
    /// The main filename.
    #[pyo3(get)]
    pub filename: Filename,
    /// The entries of the ledger (sorted).
    #[pyo3(get)]
    pub entries: Vec<Entry>,
    /// Errors that occured on parsing, booking or any later stage.
    #[pyo3(get)]
    pub errors: Vec<UroError>,
    /// The options in the file.
    #[pyo3(get)]
    pub options: BeancountOptions,
    /// Included file paths.
    #[pyo3(get)]
    pub includes: Vec<Filename>,
    /// Plugins (with optional config)
    #[pyo3(get)]
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

    /// Run the validation plugins (and add any errors).
    pub fn run_validations(&mut self) {
        self.errors.append(&mut run_validations(self));
    }
}

#[pymethods]
impl Ledger {
    /// Run the plugin with the given name (returns true if it exists)
    fn run_plugin(&mut self, plugin: &str, py: Python<'_>) -> bool {
        py.detach(|| run_named_plugin(self, plugin))
    }

    #[pyo3(name = "run_validations")]
    fn py_run_validations(&mut self, py: Python<'_>) {
        py.detach(|| self.run_validations());
    }

    /// Replace the entries of this ledger.
    fn replace_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
    }

    /// Append some error (from the Python side).
    fn add_error(&mut self, error: UroError) {
        self.errors.push(error);
    }
}
