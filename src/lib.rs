#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]

use std::path::Path;

use py_bindings::init_statics;
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
#[cfg(test)]
mod test_utils;
mod tolerances;
pub mod types;
mod util;

pub use combine::load;
pub use ledgers::Ledger;

/// Load the Beancount ledger at the given file path.
#[pyfunction]
fn load_file(filename: &str) -> PyResult<Ledger> {
    Ok(load(&Path::new(filename).try_into()?))
}

/// A Python module implemented in Rust.
#[pymodule]
fn uromyces(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_file, m)?)?;

    // Ensure that some basic types can be imported.
    init_statics(py)?;

    // Base types
    m.add_class::<types::Booking>()?;

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
    m.add_class::<types::Transaction>()?;
    m.add_class::<types::Query>()?;

    Ok(())
}
