//! Like `display_context` in Beancount, keep track of per-currency precisions.
//!
//! To infer a sensible default for the displayed precision for a certain currency, we keep track
//! of all numbers (with a matching currency) in the input files and count the number of times that
//! each display precision is used.

use std::{collections::BTreeMap, fmt::Display};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::types::{Amount, Currency, Decimal, IncompleteAmount, MetaValue, RawEntry};

/// Stats about the used precisions for a currency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrecisionStats {
    has_sign: bool,
    precisions: [u32; 29],
}

impl PrecisionStats {
    #![allow(clippy::cast_possible_truncation)]

    #[must_use]
    pub fn new() -> Self {
        Self {
            has_sign: false,
            precisions: [0; 29],
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
        self.has_sign = self.has_sign || dec.is_sign_negative();
        self.precisions[dec.scale() as usize] += 1;
    }
}

impl Default for PrecisionStats {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for PrecisionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fractional_common={} fractional_max={}",
            self.get_common(),
            self.get_max()
        )
    }
}

#[derive(Clone, Debug)]
pub struct DisplayPrecisionsStats {
    map: HashMap<Currency, PrecisionStats>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Precisions {
    has_sign: bool,
    max: u8,
    common: u8,
}

/// The summarised precisions for some currencies
///
/// This uses an ordered `BTreeMap` to allow for consistent serialisation of this as part of the
/// options
pub type DisplayPrecisions = BTreeMap<Currency, Precisions>;

impl DisplayPrecisionsStats {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Summarise the precision stats and obtain the most common and max precisions.
    #[must_use]
    pub fn get_precisions(self) -> DisplayPrecisions {
        self.map
            .iter()
            .map(|(c, p)| {
                (
                    c.clone(),
                    Precisions {
                        has_sign: p.has_sign,
                        max: p.get_max(),
                        common: p.get_common(),
                    },
                )
            })
            .collect()
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

    pub fn update_from_amount(&mut self, a: &Amount) {
        self.update(a.number, &a.currency);
    }

    fn update_from_incomplete_amount(&mut self, a: &IncompleteAmount) {
        if let Some(number) = a.number {
            if let Some(currency) = &a.currency {
                self.update(number, currency);
            }
        }
    }

    #[must_use]
    pub fn get(&self, currency: &Currency) -> Option<&PrecisionStats> {
        self.map.get(currency)
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
                RawEntry::Transaction(e) => {
                    for p in &e.postings {
                        res.update_from_incomplete_amount(&p.units);
                        if let Some(price) = &p.price {
                            res.update_from_incomplete_amount(price);
                        }
                        if let Some(cost) = &p.cost {
                            if let Some(number) = cost.number_per {
                                if let Some(currency) = &cost.currency {
                                    res.update(number, currency);
                                }
                            }
                            if let Some(number) = cost.number_total {
                                if let Some(currency) = &cost.currency {
                                    res.update(number, currency);
                                }
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

impl Default for DisplayPrecisionsStats {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for DisplayPrecisionsStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (currency, prec) in &self.map {
            writeln!(f, "{currency:>10}: {prec}")?;
        }
        write!(f, "")
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
        assert_eq!(p.get(&("EUR".into())).unwrap().get_common(), 2);
    }
}
