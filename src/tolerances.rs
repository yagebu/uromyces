//! Tolerances are used to determine if transactions balance.
use hashbrown::HashMap;

use serde::{Deserialize, Serialize};

use crate::inventory::Inventory;
use crate::options::BeancountOptions;
use crate::types::{Balance, Currency, Decimal, Posting, RawPosting};

/// Tolerances for currencies.
///
/// Consists of a map of `Decimal`s for some `Currency`s as well as a default value for all others.
/// Will mostly be inferred from the amounts in a transaction or a balance and is then used to
/// check that a transaction or balance, well, balances, that is that the remainder of all postings
/// in the transaction or the difference between the asserted and the computed balance is smaller
/// than the tolerance for each currency.
///
/// In addition to validations, the tolerances can also be used to quantize numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tolerances {
    map: HashMap<Currency, Decimal>,
    default: Decimal,
}

pub fn balance_tolerance(balance: &Balance, options: &BeancountOptions) -> Decimal {
    if let Some(explicit) = balance.tolerance {
        explicit
    } else {
        let scale = balance.amount.number.scale();
        if scale > 0 {
            let mut scaled_one = Decimal::ONE;
            scaled_one
                .set_scale(scale)
                .expect("setting scale to scale of other Decimal to work");
            // twice as lenient for balances than within transactions
            scaled_one * options.inferred_tolerance_multiplier * Decimal::TWO
        } else {
            Decimal::ZERO
        }
    }
}

impl Tolerances {
    /// Get the tolerance for a currency.
    fn get(&self, currency: &Currency) -> &Decimal {
        self.map.get(currency).unwrap_or(&self.default)
    }

    /// Quantize a decimal number according to the tolerances.
    ///
    /// We use this to get nice round numbers for interpolated posting units.
    pub fn quantize(&self, currency: &Currency, num: Decimal) -> Decimal {
        let tolerance = self.map.get(currency);
        match tolerance {
            Some(tol) => num.round_dp((tol * Decimal::TWO).normalize().scale()),
            None => num,
        }
    }

    /// Under consideration of the given tolerances, check whether all positions are small.
    #[must_use]
    pub fn is_small(&self, inv: &Inventory) -> bool {
        inv.iter()
            .all(|pos| pos.number.abs() <= *self.get(pos.currency))
    }

    /// Set from an option string like "USD:0.04".
    pub(crate) fn set_from_option(&mut self, value: &str) -> Result<(), ()> {
        let mut parts = value.split(':');
        if let Some(currency) = parts.next() {
            if let Some(tol) = parts.next() {
                let tolerance = Decimal::from_str_exact(tol).map_err(|_| ())?;
                if currency == "*" {
                    self.default = tolerance;
                } else {
                    self.map.insert(currency.into(), tolerance);
                }
                return Ok(());
            }
        }
        Err(())
    }

    /// Infer tolerance for the given number and currency.
    fn add_inferred(&mut self, number: &Decimal, currency: &Currency, multiplier: &Decimal) {
        let scale = number.scale();
        if scale > 0 {
            let mut scaled_one = Decimal::ONE;
            scaled_one
                .set_scale(scale)
                .expect("setting scale to scale of other Decimal to work");
            let mut tolerance = scaled_one * multiplier;
            self.map
                .raw_entry_mut()
                .from_key(currency)
                .and_modify(|_c, t| *t = *t.max(&mut tolerance))
                .or_insert_with(|| (currency.clone(), tolerance));
        }
    }

    /// Infer tolerances from a list of raw postings.
    pub fn infer_from_raw(postings: &[RawPosting], options: &BeancountOptions) -> Self {
        let mut tolerances = options.inferred_tolerance_default.clone();

        for posting in postings {
            if let Some(number) = &posting.units.number {
                if let Some(currency) = &posting.units.currency {
                    tolerances.add_inferred(
                        number,
                        currency,
                        &options.inferred_tolerance_multiplier,
                    );
                }
            }
        }

        tolerances
    }

    /// Infer tolerances from a list of booked postings.
    pub fn infer_from_booked(postings: &[Posting], options: &BeancountOptions) -> Self {
        let mut tolerances = options.inferred_tolerance_default.clone();

        for posting in postings {
            tolerances.add_inferred(
                &posting.units.number,
                &posting.units.currency,
                &options.inferred_tolerance_multiplier,
            );
        }

        tolerances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{c, d, postings_from_strings};

    #[test]
    fn test_simple_tolerance() {
        let postings = postings_from_strings(&["Assets:Cash 20.00 USD", "Assets:Cash 20 EUR"]);

        let options = BeancountOptions::default();
        let tolerances = Tolerances::infer_from_raw(&postings, &options);
        assert_eq!(*tolerances.get(&c("EUR")), Decimal::ZERO);
        assert_eq!(*tolerances.get(&c("USD")), d("0.005"));
    }

    #[test]
    fn test_quantize() {
        let postings = postings_from_strings(&["Assets:Cash 20.00 USD", "Assets:Cash 20 EUR"]);

        let options = BeancountOptions::default();
        let tolerances = Tolerances::infer_from_raw(&postings, &options);
        assert_eq!(
            tolerances.quantize(&c("EUR"), d("1.23456789")),
            d("1.23456789")
        );
        assert_eq!(tolerances.quantize(&c("USD"), d("1.23456789")), d("1.23"));
    }
}
