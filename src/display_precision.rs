//! Like `display_context` in Beancount, keep track of per-currency precisions.
//!
//! To infer a sensible default for the displayed precision for a certain currency, we keep track
//! of all numbers (with a matching currency) in the input files and count the number of times that
//! each display precision is used.

use std::collections::BTreeMap;

use hashbrown::HashMap;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::{Amount, Currency, Decimal, RawAmount, MetaValue, RawEntry};

const MAX_PRECISION: usize = Decimal::MAX_SCALE as usize;
const MAX_PRECISION_INDEX: usize = MAX_PRECISION + 1;

/// Stats about the used precisions for a currency.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PrecisionStats {
    has_sign: bool,
    precisions: [u32; MAX_PRECISION_INDEX],
}

impl PrecisionStats {
    #![allow(clippy::cast_possible_truncation)]

    #[must_use]
    pub fn new() -> Self {
        Self {
            has_sign: false,
            precisions: [0; MAX_PRECISION_INDEX],
        }
    }

    /// Get the maximum number of used decimal digits.
    fn get_max(&self) -> u8 {
        let mut max_index = 0;
        for (index, count) in self.precisions.iter().enumerate() {
            if count > &0 {
                max_index = index;
            }
        }
        max_index as u8
    }

    /// Get the most common number of decimal digits.
    fn get_common(&self) -> u8 {
        let mut max_index = 0;
        let mut max_count = 0;
        for (index, count) in self.precisions.iter().enumerate() {
            if count > &max_count {
                max_count = *count;
                max_index = index;
            }
        }
        max_index as u8
    }

    /// Update stats with the given number.
    fn update(&mut self, dec: Decimal) {
        self.has_sign = self.has_sign || !dec.is_sign_positive();
        let scale = dec.scale() as usize;
        assert!(scale < MAX_PRECISION_INDEX);
        self.precisions[scale] += 1;
    }
}

#[derive(Clone, Debug)]
struct DisplayPrecisionsStats {
    map: HashMap<Currency, PrecisionStats>,
}

/// Precisions for a currency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Precisions {
    has_sign: bool,
    max: u8,
    common: u8,
}

#[pymethods]
impl Precisions {
    fn __repr__(&self) -> String {
        format!(
            "Precisions(has_sign={}, max={}, common={})",
            self.has_sign, self.max, self.common
        )
    }
}

impl From<PrecisionStats> for Precisions {
    fn from(value: PrecisionStats) -> Self {
        Precisions {
            has_sign: value.has_sign,
            max: value.get_max(),
            common: value.get_common(),
        }
    }
}

impl<'py> IntoPyObject<'py> for &Precisions {
    type Target = Precisions;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.clone().into_pyobject(py)
    }
}

/// The summarised precisions for some currencies
///
/// This uses an ordered `BTreeMap` to allow for consistent serialisation of this as part of the
/// options
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, IntoPyObjectRef)]
pub struct DisplayPrecisions(BTreeMap<Currency, Precisions>);

impl DisplayPrecisions {
    /// Create precision stats and summarise them to obtain the most common and max precisions.
    #[must_use]
    pub fn from_raw_entries(entries: &[RawEntry]) -> Self {
        DisplayPrecisionsStats::from_raw_entries(entries).into()
    }
}

impl From<DisplayPrecisionsStats> for DisplayPrecisions {
    fn from(value: DisplayPrecisionsStats) -> Self {
        Self(value.map.into_iter().map(|(c, p)| (c, p.into())).collect())
    }
}

impl DisplayPrecisionsStats {
    #[must_use]
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn update(&mut self, number: Decimal, currency: &Currency) {
        self.map
            .raw_entry_mut()
            .from_key(currency)
            .and_modify(|_c, currency_precision| {
                currency_precision.update(number);
            })
            .or_insert_with(|| {
                let mut currency_precision = PrecisionStats::new();
                currency_precision.update(number);
                (currency.clone(), currency_precision)
            });
    }

    fn update_from_amount(&mut self, a: &Amount) {
        self.update(a.number, &a.currency);
    }

    fn update_from_incomplete_amount(&mut self, a: &RawAmount) {
        if let Some(number) = a.number
            && let Some(currency) = &a.currency
        {
            self.update(number, currency);
        }
    }

    #[must_use]
    pub fn from_raw_entries(entries: &[RawEntry]) -> Self {
        let mut res = Self::new();
        for entry in entries {
            match entry {
                RawEntry::Balance(e) => res.update_from_amount(&e.amount),
                RawEntry::Custom(e) => {
                    for v in &e.values {
                        if let MetaValue::Amount(a) = &v.0 {
                            res.update_from_amount(a);
                        }
                    }
                }
                RawEntry::Price(e) => res.update_from_amount(&e.amount),
                RawEntry::RawTransaction(e) => {
                    for p in &e.postings {
                        res.update_from_incomplete_amount(&p.units);
                        if let Some(price) = &p.price {
                            res.update_from_incomplete_amount(price);
                        }
                        if let Some(cost) = &p.cost {
                            if let Some(number) = cost.number_per
                                && let Some(currency) = &cost.currency
                            {
                                res.update(number, currency);
                            }
                            if let Some(number) = cost.number_total
                                && let Some(currency) = &cost.currency
                            {
                                res.update(number, currency);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{a, d};

    use super::*;

    #[test]
    fn test_precision_stats() {
        let a0 = d("200");
        let a2 = d("2.00");
        let a3 = d("0.200");

        let mut p = PrecisionStats::new();

        p.update(a0);
        assert_eq!(p.get_max(), 0);

        p.update(a2);
        p.update(a2);
        p.update(a2);
        assert_eq!(p.get_common(), 2);
        assert_eq!(p.get_max(), 2);

        p.update(a3);
        p.update(a3);
        assert_eq!(p.get_common(), 2);
        assert_eq!(p.get_max(), 3);

        p.update(a3);
        p.update(a3);
        assert_eq!(p.get_common(), 3);
        assert_eq!(p.get_max(), 3);

        p.update(a3);
        p.update(d("0.1234567890123456789012345678"));
        assert_eq!(p.get_common(), 3);
        assert_eq!(p.get_max(), 28);
    }

    #[test]
    fn test_currency_precisions() {
        let c_eur0 = a("200 EUR");
        let c_eur2 = a("2.00 EUR");

        let mut p = DisplayPrecisionsStats::new();

        p.update_from_amount(&c_eur0);
        p.update_from_amount(&c_eur0);
        p.update_from_amount(&c_eur2);
        p.update_from_amount(&c_eur2);
        p.update_from_amount(&c_eur2);
        let eur: Currency = "EUR".into();
        assert_eq!(p.map.get(&eur).unwrap().get_common(), 2);
    }
}
