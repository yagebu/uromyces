//! Like `display_context` in Beancount, this keeps tracks of the precisions used for currencies.
//!
//! To infer a sensible default for the displayed precision for a certain currency, we keep track
//! of all numbers (with a matching currency) in the input files and count the number of times that
//! each display precision is used.

use hashbrown::HashMap;
use std::fmt::Display;

use crate::types::{Amount, Currency, Decimal};

/// Stats about the used precisions for a currency.
#[derive(Debug)]
pub struct PrecisionStats {
    has_sign: bool,
    precisions: [u32; 28],
}

impl PrecisionStats {
    #![allow(clippy::cast_possible_truncation)]

    #[must_use]
    pub fn new() -> Self {
        Self {
            has_sign: false,
            precisions: [0; 28],
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

#[derive(Debug)]
pub struct DisplayPrecisions {
    map: HashMap<Currency, PrecisionStats>,
}

impl DisplayPrecisions {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn update_from_amount(&mut self, a: &Amount) {
        self.map
            .raw_entry_mut()
            .from_key(&a.currency)
            .and_modify(|_c, currency_precision| {
                currency_precision.update(a.number);
            })
            .or_insert_with(|| {
                let mut currency_precision = PrecisionStats::new();
                currency_precision.update(a.number);
                (a.currency.clone(), currency_precision)
            });
    }

    #[must_use]
    pub fn get(&self, currency: &Currency) -> Option<&PrecisionStats> {
        self.map.get(currency)
    }
}

impl Default for DisplayPrecisions {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for DisplayPrecisions {
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
    }

    #[test]
    fn test_currency_precisions() {
        let c_eur0 = a("200 EUR");
        let c_eur2 = a("2.00 EUR");

        let mut p = DisplayPrecisions::new();

        p.update_from_amount(&c_eur0);
        p.update_from_amount(&c_eur0);
        p.update_from_amount(&c_eur2);
        p.update_from_amount(&c_eur2);
        p.update_from_amount(&c_eur2);
        assert_eq!(p.get(&("EUR".into())).unwrap().get_common(), 2);
    }
}
