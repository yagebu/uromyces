use std::fmt::{Debug, Display};
use std::ops::Neg;
use std::str::FromStr;

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Currency, Decimal, DecimalPyWrapper};

/// An amount.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Amount {
    /// The number of units in this amount.
    pub number: Decimal,
    /// The currency of the units in this amount.
    #[pyo3(get)]
    pub currency: Currency,
}

#[pymethods]
impl Amount {
    #[getter]
    fn number(&self) -> DecimalPyWrapper {
        DecimalPyWrapper(self.number)
    }
}

impl Neg for Amount {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            number: -self.number,
            currency: self.currency,
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.number, self.currency)
    }
}

impl FromStr for Amount {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let handle_err = |_| ();
        Ok(Self {
            number: Decimal::from_str_exact(parts.next().unwrap_or_default())
                .map_err(handle_err)?,
            currency: parts.next().unwrap_or_default().into(),
        })
    }
}

/// An amount, where one or both of number and currency might still be missing.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct IncompleteAmount {
    pub number: Option<Decimal>,
    pub currency: Option<Currency>,
}
