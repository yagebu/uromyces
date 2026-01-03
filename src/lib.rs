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
pub mod summarize;
#[cfg(test)]
mod test_utils;
mod tolerances;
pub mod types;
mod util;

pub use combine::{load, load_string};
pub use ledgers::Ledger;

/// [pymodule] The uromyces.uromyces Python extension module.
#[pymodule]
mod uromyces {
    use pyo3::prelude::*;
    use pyo3::types::PyMapping;

    use crate::options::BeancountOptions;
    use crate::types::{AbsoluteUTF8Path, Filename};
    use crate::{summarize, types};

    // Base types
    #[pymodule_export]
    use crate::Ledger;
    #[pymodule_export]
    use crate::types::{Amount, Booking, Cost, CustomValue, EntryHeader, Posting};

    // Entry types
    #[pymodule_export]
    use crate::types::{
        Balance, Close, Commodity, Custom, Document, Event, Note, Open, Pad, Price, Query,
        Transaction,
    };

    /// Load the Beancount ledger at the given file path.
    #[pyfunction]
    fn load_file(filename: AbsoluteUTF8Path) -> Ledger {
        crate::load(filename)
    }

    /// Load a Beancount ledger from the given string.
    #[pyfunction]
    fn load_string(string: &str, filename: Filename) -> Ledger {
        crate::load_string(string, filename)
    }

    /// Clamp the entries to the given interval.
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

    #[pymodule_init]
    fn init_uromyces(m: &Bound<'_, PyModule>) -> PyResult<()> {
        pyo3_log::init();

        PyMapping::register::<types::EntryHeader>(m.py())?;

        Ok(())
    }
}
