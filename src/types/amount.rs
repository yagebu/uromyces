use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Neg;
use std::str::FromStr;

use pyo3::{intern, prelude::*};
use serde::{Deserialize, Serialize};

use crate::types::{Cost, Currency, Decimal};

/// An amount.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, str, hash, module = "uromyces", skip_from_py_object)]
pub struct Amount {
    /// The number of units in this amount.
    #[pyo3(get)]
    pub number: Decimal,
    /// The currency of the units in this amount.
    #[pyo3(get)]
    pub currency: Currency,
}

impl Amount {
    /// Create an amount from a number and currency.
    #[must_use]
    pub fn new(number: Decimal, currency: Currency) -> Self {
        Self { number, currency }
    }

    #[must_use]
    pub fn from_cost(cost: &Cost) -> Self {
        Self {
            number: cost.number,
            currency: cost.currency.clone(),
        }
    }
}

#[pymethods]
impl Amount {
    #[new]
    fn __new__(number: Decimal, currency: Currency) -> Self {
        Self { number, currency }
    }
    fn __repr__(&self) -> String {
        format!("{} {}", self.number, self.currency)
    }
}

impl<'py> IntoPyObject<'py> for &Amount {
    type Target = Amount;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.clone().into_pyobject(py)
    }
}

impl<'py> FromPyObject<'_, 'py> for Amount {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(amount) = obj.cast::<Amount>() {
            Ok(amount.get().clone())
        } else {
            let number = obj.getattr(intern!(obj.py(), "number"))?;
            let currency = obj.getattr(intern!(obj.py(), "currency"))?;

            Ok(Amount {
                number: number.extract()?,
                currency: currency.extract()?,
            })
        }
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
        let raw_number = parts.next().ok_or(())?;
        let raw_currency = parts.next().ok_or(())?;
        if parts.next().is_some() {
            return Err(());
        }
        Ok(Self {
            number: Decimal::from_str_exact(raw_number).map_err(|_| ())?,
            currency: raw_currency.into(),
        })
    }
}

/// An amount, where one or both of number and currency might still be missing.
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct IncompleteAmount {
    pub number: Option<Decimal>,
    pub currency: Option<Currency>,
}

impl Display for IncompleteAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self {
                number: Some(n),
                currency: Some(c),
            } => write!(f, "{n} {c}"),
            Self {
                number: None,
                currency: Some(c),
            } => write!(f, "{c}"),
            Self {
                number: Some(n),
                currency: None,
            } => write!(f, "{n}"),
            Self {
                number: None,
                currency: None,
            } => write!(f, ""),
        }
    }
}

impl From<Amount> for IncompleteAmount {
    fn from(amount: Amount) -> Self {
        Self {
            number: Some(amount.number),
            currency: Some(amount.currency),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_from_string() {
        let one = Decimal::ONE;
        let eur = Currency::from("EUR");
        assert_eq!(Amount::from_str("1 EUR"), Ok(Amount::new(one, eur.clone())));
        assert_eq!(
            Amount::from_str("1    EUR"),
            Ok(Amount::new(one, eur.clone()))
        );
        assert_eq!(Amount::from_str("1    EUR   asdf"), Err(()));
        assert_eq!(Amount::from_str("1"), Err(()));
        assert_eq!(Amount::from_str("EUR"), Err(()));
    }
}
