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
//! inventory.add_amount(Amount::from_str("10 USD").unwrap());
//! inventory.add_amount(Amount::from_str("10 USD").unwrap());
//! assert_eq!(inventory.len(), 1);
//! assert_eq!(inventory.currencies(), vec![&Currency::from("USD")]);
//! ```
//!
use hashbrown::HashMap;

use crate::types::{Amount, Cost, Currency, Decimal};

/// A single item in an inventory is keyed by currency and optional cost.
#[derive(Debug, Hash, Eq, PartialEq)]
struct InventoryKey {
    currency: Currency,
    cost: Option<Cost>,
}

/// An inventory, basically a map of `[Currency]`, `[Option<Cost>]` pairs to `[Decimals]`.
#[derive(Debug)]
pub struct Inventory {
    map: HashMap<InventoryKey, Decimal>,
}

/// An inventory position of number, currency and optional cost.
pub struct Position<'inv> {
    /// The number of units of this position.
    pub number: &'inv Decimal,
    /// The currency that this position is in.
    pub currency: &'inv Currency,
    /// The cost, if this position is held at cost.
    pub cost: &'inv Option<Cost>,
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

impl Inventory {
    /// Create an empty inventory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
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

    /// Add an amount to this inventory.
    pub fn add_amount(&mut self, amount: Amount) {
        self.add_position(amount, None);
    }

    /// An iterator over all positions in this inventory.
    ///
    /// Just like when iterating over the underlying `HashMap`, the items contain borrowed values.
    pub fn iter(&self) -> impl Iterator<Item = Position> {
        self.map.iter().map(|(k, v)| Position {
            number: v,
            currency: &k.currency,
            cost: &k.cost,
        })
    }

    /// An iterator over all positions with cost in this inventory.
    ///
    /// This is just like the `.iter()` function above but skips all positions that do not have a cost
    /// and has a iterator item types that ensures this.
    /// Just like when iterating over the underlying `HashMap`, the items contain borrowed values.
    pub fn iter_with_cost(&self) -> impl Iterator<Item = PositionWithCost> {
        self.map.iter().filter_map(|(k, v)| {
            k.cost.as_ref().map(|cost| PositionWithCost {
                number: v,
                currency: &k.currency,
                cost,
            })
        })
    }

    /// Add a position to the inventory.
    pub fn add_position(&mut self, amount: Amount, cost: Option<Cost>) {
        let key = InventoryKey {
            currency: amount.currency,
            cost,
        };
        let pos = self.map.get_mut(&key);
        if let Some(num) = pos {
            *num += amount.number;
            if *num == Decimal::ZERO {
                self.map.remove(&key);
            }
        } else {
            self.map.insert(key, amount.number);
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

impl FromIterator<Amount> for Inventory {
    fn from_iter<T: IntoIterator<Item = Amount>>(iter: T) -> Self {
        let mut inv = Self::new();
        for a in iter {
            inv.add_amount(a);
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
        inv.add_amount(a("2.0 EUR"));
        assert!(!inv.is_empty());
        inv.add_amount(a("-2.0 EUR"));
        assert!(inv.is_empty());
    }

    #[test]
    fn test_inventory_get_currencies() {
        let mut inv = Inventory::new();
        inv.add_amount(a("2.0 EUR"));
        assert_eq!(inv.currencies(), vec!["EUR"]);
        inv.add_amount(a("2.0 USD"));
        let mut currencies = inv.currencies();
        currencies.sort();
        assert_eq!(currencies, vec!["EUR", "USD"]);
    }

    #[test]
    fn test_inventory_is_reduced_by() {
        let mut inv = Inventory::new();
        inv.add_amount(a("2.0 EUR"));
        inv.add_amount(a("2.0 USD"));
        assert!(!inv.is_reduced_by(&a("2 ASDF")));
        assert!(!inv.is_reduced_by(&a("2 USD")));
        assert!(inv.is_reduced_by(&a("-2 USD")));
    }
}
