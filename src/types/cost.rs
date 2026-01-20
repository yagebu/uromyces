use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::repr::PyRepresentation;
use crate::types::{BoxStr, Currency, Date, Decimal};

#[derive(
    Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, FromPyObject, IntoPyObjectRef,
)]
#[serde(transparent)]
pub struct CostLabel(BoxStr);

impl Display for CostLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CostLabel {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for CostLabel {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

/// A cost (basically an Amount + date and label).
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces", skip_from_py_object)]
pub struct Cost {
    /// The per-unit cost.
    pub number: Decimal,
    /// The currency.
    pub currency: Currency,
    /// The date that this lot was created.
    ///
    /// This can be provided in the input but will be filled in by the transaction date
    /// if not provided automatically.
    pub date: Date,
    /// An optional label to identify a position.
    pub label: Option<CostLabel>,
}

impl Cost {
    #[must_use]
    pub fn new(number: Decimal, currency: Currency, date: Date, label: Option<CostLabel>) -> Self {
        Self {
            number,
            currency,
            date,
            label,
        }
    }
}

impl Display for Cost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}, {}", self.number, self.currency, self.date)?;
        if let Some(label) = &self.label {
            write!(f, ", {label}")?;
        }
        Ok(())
    }
}

#[pymethods]
impl Cost {
    #[new]
    #[pyo3(signature = (number, currency, date, label=None))]
    fn __new__(number: Decimal, currency: Currency, date: Date, label: Option<CostLabel>) -> Self {
        Self::new(number, currency, date, label)
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
    fn __repr__(&self) -> String {
        self.py_repr()
    }
}

impl<'py> FromPyObject<'_, 'py> for Cost {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(a) = obj.cast::<Self>() {
            Ok(a.get().clone())
        } else {
            let py = obj.py();
            let number = obj.getattr(pyo3::intern!(py, "number"))?;
            let currency = obj.getattr(pyo3::intern!(py, "currency"))?;
            let date = obj.getattr(pyo3::intern!(py, "date"))?;
            let label = obj.getattr(pyo3::intern!(py, "label"))?;

            Ok(Cost::new(
                number.extract()?,
                currency.extract()?,
                date.extract()?,
                label.extract()?,
            ))
        }
    }
}

/// A possibly incomplete cost as specified in the Beancount file.
#[allow(clippy::module_name_repetitions)]
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces", skip_from_py_object)]
pub struct CostSpec {
    /// The per-unit cost.
    pub number_per: Option<Decimal>,
    /// The total cost.
    pub number_total: Option<Decimal>,
    /// The currency.
    pub currency: Option<Currency>,
    /// The date that this lot was created.
    pub date: Option<Date>,
    /// An optional label to identify a position.
    pub label: Option<CostLabel>,
    /// Unsupported, like in Beancount v2.
    pub merge: bool,
}

#[pymethods]
impl CostSpec {
    #[new]
    fn __new__(
        number_per: Option<Decimal>,
        number_total: Option<Decimal>,
        currency: Option<Currency>,
        date: Option<Date>,
        label: Option<CostLabel>,
        merge: Option<bool>,
    ) -> Self {
        Self {
            number_per,
            number_total,
            currency,
            date,
            label,
            merge: merge.unwrap_or(false),
        }
    }
    fn __repr__(&self) -> String {
        self.py_repr()
    }
}

impl From<Cost> for CostSpec {
    fn from(cost: Cost) -> Self {
        Self {
            number_per: Some(cost.number),
            number_total: None,
            currency: Some(cost.currency),
            date: Some(cost.date),
            label: cost.label,
            merge: false,
        }
    }
}
impl From<&Cost> for CostSpec {
    fn from(cost: &Cost) -> Self {
        Self {
            number_per: Some(cost.number),
            number_total: None,
            currency: Some(cost.currency.clone()),
            date: Some(cost.date),
            label: cost.label.clone(),
            merge: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_to_string() {
        let one = Decimal::ONE;
        let eur = Currency::from("EUR");
        let cost = Cost::new(
            one,
            eur.clone(),
            Date::try_from_str("2012-12-12").unwrap(),
            None,
        );
        assert_eq!(cost.to_string(), "1 EUR, 2012-12-12");
        let cost_with_label = Cost::new(
            one,
            eur,
            Date::try_from_str("2012-12-12").unwrap(),
            Some("lot-1".into()),
        );
        assert_eq!(cost_with_label.to_string(), "1 EUR, 2012-12-12, lot-1");
    }
}
