use std::fmt::{Debug, Display};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::interning::CurrencyInternedString;

/// A currency name.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromPyObject,
    IntoPyObjectRef,
)]
pub struct Currency(CurrencyInternedString);

impl Currency {
    /// Create a currency name instance.
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(CurrencyInternedString::new(s))
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_display_and_debug() {
        let currency = Currency::new("USD");
        assert_eq!(format!("{currency}"), "USD");
        assert_eq!(format!("{currency:?}"), "Currency(\"USD\")");
    }
}
