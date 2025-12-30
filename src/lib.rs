#![doc = include_str!("../README.md")]
// enable some stricter lint groups
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
// enable some additional lint rules
#![warn(clippy::unwrap_used)]
// allow some rules enabled by the rules above
// Stylistic, sometimes preferred to have the name in some function again.
#![allow(clippy::module_name_repetitions)]
// Warns on Deserialize on pyo3 structs which should be fine.
#![allow(clippy::unsafe_derive_deserialize)]

use options::BeancountOptions;
use pyo3::prelude::*;

pub mod booking;
mod combine;
mod conversions;
pub mod display_precision;
pub mod errors;
pub mod inventory;
mod ledgers;
pub mod options;
pub mod parse;
mod plugins;
mod py_bindings;
pub mod summarize;
#[cfg(test)]
mod test_utils;
mod tolerances;
pub mod types;
mod util;

pub use combine::{load, load_string};
pub use ledgers::Ledger;
use py_bindings::init_statics;

use crate::types::{AbsoluteUTF8Path, Filename};

/// [pyfunction] Load the Beancount ledger at the given file path.
#[pyfunction(name = "load_file")]
fn py_load_file(filename: AbsoluteUTF8Path) -> Ledger {
    load(filename)
}

/// [pyfunction] Load a Beancount ledger from the given string.
#[pyfunction(name = "load_string")]
fn py_load_string(string: &str, filename: Filename) -> Ledger {
    load_string(string, filename)
}

/// [pyfunction] Clamp the entries to the given interval.
#[pyfunction]
#[allow(clippy::needless_pass_by_value)]
fn summarize_clamp(
    entries: Vec<types::Entry>,
    begin_date: types::Date,
    end_date: types::Date,
    options: &BeancountOptions,
) -> Vec<types::Entry> {
    summarize::clamp(
        &entries,
        begin_date,
        end_date,
        &options.get_summarization_accounts(),
    )
}

/// [pymodule] The uromyces.uromyces Python extension module.
#[pymodule]
fn uromyces(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    // Ensure that some basic types can be imported.
    init_statics(py)?;

    m.add_function(wrap_pyfunction!(py_load_file, m)?)?;
    m.add_function(wrap_pyfunction!(py_load_string, m)?)?;
    m.add_function(wrap_pyfunction!(summarize_clamp, m)?)?;

    // Base types
    m.add_class::<types::Amount>()?;
    m.add_class::<types::Booking>()?;
    m.add_class::<types::Cost>()?;
    m.add_class::<types::CustomValue>()?;
    m.add_class::<types::EntryHeader>()?;
    m.add_class::<types::Posting>()?;

    // Ledger
    m.add_class::<Ledger>()?;

    // Entry types
    m.add_class::<types::Balance>()?;
    m.add_class::<types::Close>()?;
    m.add_class::<types::Commodity>()?;
    m.add_class::<types::Custom>()?;
    m.add_class::<types::Document>()?;
    m.add_class::<types::Event>()?;
    m.add_class::<types::Note>()?;
    m.add_class::<types::Open>()?;
    m.add_class::<types::Pad>()?;
    m.add_class::<types::Price>()?;
    m.add_class::<types::Query>()?;
    m.add_class::<types::Transaction>()?;

    Ok(())
}
