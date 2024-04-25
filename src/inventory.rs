//! Inventories.
//!
//! An [`Inventory`] maps currencies, optionally with an attached Cost, to decimal numbers.
//! Inventories are the central utility to balance transactions, book reductions, and compute
//! account balances.
//!
//! # Examples
//!
//! ```
//! use std::str::FromStr;
//!
//! use uromyces::inventory::Inventory;
//! use uromyces::types::{Amount, Currency};
//!
//! let mut inventory = Inventory::new();
//! assert!(inventory.is_empty());
//!
//! inventory.add_position(&Amount::from_str("10 USD").unwrap());
//! inventory.add_position(&Amount::from_str("10 USD").unwrap());
//! assert_eq!(inventory.len(), 1);
//! assert_eq!(inventory.currencies(), vec![&Currency::from("USD")]);
//! let sum = Amount::from_str("20 USD").unwrap();
//! assert_eq!(inventory.get(sum.currency, None), Some(sum.number));
//! ```
//!
use indexmap::IndexMap;
use rust_decimal::prelude::Signed;

use crate::types::{Amount, Cost, Currency, Decimal, Posting};

/// A single item in an inventory is keyed by currency and optional cost.
#[derive(Debug, Hash, Eq, PartialEq)]
struct InventoryKey {
    currency: Currency,
    cost: Option<Cost>,
}

#[derive(Debug, PartialEq)]
pub enum BookingResult {
    /// A new lot was created.
    CREATED,
    /// An existing lot was reduced.
    REDUCED,
    /// An existing lot was augmented.
    AUGMENTED,
    /// A change of zero was ignored.
    IGNORED,
}

/// A position contains units (number, currency) and an optional cost.
pub trait Position {
    /// Get the number of this position.
    #[must_use]
    fn number(&self) -> Decimal;
    /// Get the currency of this position.
    #[must_use]
    fn currency(&self) -> Currency;
    /// Get the units of this position.
    ///
    /// This default implementation uses the number and currency fns.
    #[must_use]
    fn units(&self) -> Amount {
        Amount::new(self.number(), self.currency())
    }
    /// Get the cost of this position.
    #[must_use]
    fn cost(&self) -> Option<Cost>;
    /// Get the total cost of this position.
    #[must_use]
    fn total_cost(&self) -> Amount {
        if let Some(cost) = self.cost() {
            Amount::new(self.number() * cost.number, cost.currency.clone())
        } else {
            self.units()
        }
    }
}

impl Position for Amount {
    fn number(&self) -> Decimal {
        self.number
    }
    fn currency(&self) -> Currency {
        self.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        None
    }
}
impl Position for &Amount {
    fn number(&self) -> Decimal {
        self.number
    }
    fn currency(&self) -> Currency {
        self.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        None
    }
}
impl Position for Posting {
    fn number(&self) -> Decimal {
        self.units.number
    }
    fn currency(&self) -> Currency {
        self.units.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        self.cost.clone()
    }
}
impl Position for &Posting {
    fn number(&self) -> Decimal {
        self.units.number
    }
    fn currency(&self) -> Currency {
        self.units.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        self.cost.clone()
    }
}

/// An inventory position of number, currency and optional cost.
#[allow(clippy::module_name_repetitions)]
pub struct InventoryPosition<'inv> {
    /// The number of units of this position.
    pub number: &'inv Decimal,
    /// The currency that this position is in.
    pub currency: &'inv Currency,
    /// The cost, if this position is held at cost.
    pub cost: &'inv Option<Cost>,
}
impl Position for InventoryPosition<'_> {
    fn number(&self) -> Decimal {
        *self.number
    }
    fn currency(&self) -> Currency {
        self.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        self.cost.clone()
    }
}

/// An inventory position of number, currency and cost, when filtering on positions with cost.
pub struct PositionWithCost<'inv> {
    /// The number of units of this position.
    pub number: &'inv Decimal,
    /// The currency that this position is in.
    pub currency: &'inv Currency,
    /// The cost.
    pub cost: &'inv Cost,
}
impl Position for PositionWithCost<'_> {
    fn number(&self) -> Decimal {
        *self.number
    }
    fn currency(&self) -> Currency {
        self.currency.clone()
    }
    fn cost(&self) -> Option<Cost> {
        Some(self.cost.clone())
    }
}

/// An inventory, basically a map of [`Currency`], Option<[`Cost`]> pairs to [`Decimal`]s.
#[derive(Debug)]
pub struct Inventory {
    map: IndexMap<InventoryKey, Decimal>,
}

impl Inventory {
    /// Create an empty inventory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: IndexMap::new(),
        }
    }

    /// Whether this inventory is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The number of positions in this inventory.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// The number of positions in this inventory.
    #[must_use]
    pub fn get(&self, currency: Currency, cost: Option<Cost>) -> Option<Decimal> {
        self.map.get(&InventoryKey { currency, cost }).copied()
    }

    /// Get the list of currencies contained in this inventory.
    #[must_use]
    pub fn currencies(&self) -> Vec<&Currency> {
        self.map.keys().map(|key| &key.currency).collect()
    }

    /// An iterator over all positions in this inventory.
    ///
    /// Just like when iterating over the underlying [`IndexMap`], the items contain borrowed values.
    pub fn iter(&self) -> impl Iterator<Item = InventoryPosition> {
        self.map.iter().map(|(k, v)| InventoryPosition {
            number: v,
            currency: &k.currency,
            cost: &k.cost,
        })
    }

    /// An iterator over all positions with cost in this inventory.
    ///
    /// This is just like the `.iter()` function above but skips all positions that do not have a cost
    /// and has a iterator item types that ensures this.
    /// Just like when iterating over the underlying [`IndexMap`], the items contain borrowed values.
    pub fn iter_with_cost(&self) -> impl Iterator<Item = PositionWithCost> {
        self.map.iter().filter_map(|(k, v)| {
            k.cost.as_ref().map(|cost| PositionWithCost {
                number: v,
                currency: &k.currency,
                cost,
            })
        })
    }

    /// Add all positions from another inventory.
    pub fn add_inventory(&mut self, inv: &Inventory) {
        for pos in inv.iter() {
            self.add_position(&pos);
        }
    }

    /// Add a position to the inventory.
    pub fn add_position(&mut self, pos: &impl Position) -> BookingResult {
        let key = InventoryKey {
            currency: pos.currency(),
            cost: pos.cost(),
        };
        let number = pos.number();
        let pos = self.map.get_mut(&key);
        if let Some(num) = pos {
            let result_type = if num.signum() == number.signum() {
                BookingResult::AUGMENTED
            } else {
                BookingResult::REDUCED
            };
            *num += number;
            if *num == Decimal::ZERO {
                self.map.swap_remove(&key);
            };
            result_type
        } else if number.is_zero() {
            // this matches the Beancount logic but is a bit confusing since we only return
            // the ignored value in case the position did not yet exist in the inventory.
            BookingResult::IGNORED
        } else {
            self.map.insert(key, number);
            BookingResult::CREATED
        }
    }

    /// Check whether the given amount could reduce this inventory (without checking costs)
    #[must_use]
    pub fn is_reduced_by(&self, amount: &Amount) -> bool {
        if amount.number.is_zero() {
            false
        } else {
            let amount_is_positive = amount.number.is_sign_positive();
            self.iter().any(|pos| {
                pos.currency == &amount.currency
                    && amount_is_positive != pos.number.is_sign_positive()
            })
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FromIterator<T> for Inventory
where
    T: Position,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut inv = Self::new();
        for a in iter {
            inv.add_position(&a);
        }
        inv
    }
}

#[cfg(test)]
mod tests {

    use crate::test_utils::a;

    use super::*;

    #[test]
    fn test_inventory_is_empty() {
        let mut inv = Inventory::new();
        assert!(inv.is_empty());
        let res = inv.add_position(&a("2.0 EUR"));
        assert_eq!(res, BookingResult::CREATED);
        assert!(!inv.is_empty());
        let res = inv.add_position(&a("0 EUR"));
        assert_eq!(res, BookingResult::REDUCED);
        let res = inv.add_position(&a("1.0 EUR"));
        assert_eq!(res, BookingResult::AUGMENTED);
        assert!(!inv.is_empty());
        let res = inv.add_position(&a("-3.0 EUR"));
        assert_eq!(res, BookingResult::REDUCED);
        let res = inv.add_position(&a("0 USD"));
        assert_eq!(res, BookingResult::IGNORED);
        let res = inv.add_position(&a("0 EUR"));
        assert_eq!(res, BookingResult::IGNORED);
        assert!(inv.is_empty());
    }

    #[test]
    fn test_inventory_get_currencies() {
        let mut inv = Inventory::new();
        inv.add_position(&a("2.0 EUR"));
        assert_eq!(inv.currencies(), vec!["EUR"]);
        inv.add_position(&a("2.0 USD"));
        let mut currencies = inv.currencies();
        currencies.sort();
        assert_eq!(currencies, vec!["EUR", "USD"]);
    }

    #[test]
    fn test_inventory_is_reduced_by() {
        let mut inv = Inventory::new();
        inv.add_position(&a("2.0 EUR"));
        inv.add_position(&a("2.0 USD"));
        assert!(!inv.is_reduced_by(&a("2 ASDF")));
        assert!(!inv.is_reduced_by(&a("2 USD")));
        assert!(inv.is_reduced_by(&a("-2 USD")));
    }
}
