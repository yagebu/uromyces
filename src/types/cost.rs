use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::py_bindings::{decimal_to_py, py_to_decimal};

use super::{Currency, Date, Decimal};

/// A cost (basically an Amount + date and label).
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Cost {
    /// The per-unit cost.
    pub number: Decimal,
    /// The currency.
    #[pyo3(get)]
    pub currency: Currency,
    /// The date that this lot was created.
    ///
    /// This can be provided in the input but will be filled in by the transaction date
    /// if not provided automatically.
    #[pyo3(get)]
    pub date: Date,
    /// An optional label to identify a position.
    #[pyo3(get)]
    pub label: Option<String>,
}

impl Cost {
    #[must_use]
    pub fn new(number: Decimal, currency: Currency, date: Date, label: Option<String>) -> Self {
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
        };
        Ok(())
    }
}

#[pymethods]
impl Cost {
    #[new]
    #[pyo3(signature = (number, currency, date, label=None))]
    fn __new__(
        #[pyo3(from_py_with = "py_to_decimal")] number: Decimal,
        currency: Currency,
        date: Date,
        label: Option<String>,
    ) -> Self {
        Self {
            number,
            currency,
            date,
            label,
        }
    }
    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
    fn __ne__(&self, other: &Self) -> bool {
        self != other
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
    #[getter]
    fn number<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        decimal_to_py(py, self.number)
    }
}

/// Convert from a Python object which has the correct attributes.
pub fn cost_from_py(ob: &Bound<'_, PyAny>) -> PyResult<Cost> {
    if let Ok(a) = ob.downcast::<Cost>() {
        Ok(a.get().clone())
    } else {
        let py = ob.py();
        let number = ob.getattr(pyo3::intern!(py, "number"))?;
        let currency = ob.getattr(pyo3::intern!(py, "currency"))?;
        let date = ob.getattr(pyo3::intern!(py, "date"))?;
        let label = ob.getattr(pyo3::intern!(py, "label"))?;

        Ok(Cost {
            number: py_to_decimal(&number)?,
            currency: currency.extract()?,
            date: date.extract()?,
            label: label.extract()?,
        })
    }
}

pub fn option_cost_from_py(ob: &Bound<'_, PyAny>) -> PyResult<Option<Cost>> {
    Ok(if ob.is_none() {
        None
    } else {
        Some(cost_from_py(ob)?)
    })
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

/// A possibly incomplete cost as specified in the Beancount file.
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
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
    pub label: Option<String>,
    /// Unsupported, like in Beancount v2.
    pub merge: bool,
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
