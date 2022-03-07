use serde::{Deserialize, Serialize};

use super::{Currency, Date, Decimal};

/// A cost (basically an Amount + date and label).
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// The per-unit cost.
    pub number: Decimal,
    /// The currency.
    pub currency: Currency,
    /// The date that this lot was created.
    pub date: Date,
    /// An optional label to identify a position.
    pub label: Option<String>,
}

/// A possibly incomplete cost as specified in the Beancount file.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
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

impl From<&Cost> for CostSpec {
    fn from(cost: &Cost) -> Self {
        CostSpec {
            number_per: Some(cost.number),
            number_total: None,
            currency: Some(cost.currency.clone()),
            date: Some(cost.date),
            label: cost.label.clone(),
            merge: false,
        }
    }
}
