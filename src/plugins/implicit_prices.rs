use hashbrown::{HashMap, HashSet};

use crate::Ledger;
use crate::errors::UroError;
use crate::inventory::{BookingResult, Inventory};
use crate::types::{Amount, Entry, EntryHeader, Price};

const META_KEY: &str = "__implicit_prices__";

/// Add implicitly defined prices.
pub fn add(ledger: &Ledger) -> (Vec<Entry>, Vec<UroError>) {
    let mut new_prices = Vec::new();

    let mut balances = HashMap::new();
    let mut unique_prices = HashSet::new();

    for e in &ledger.entries {
        if let Entry::Transaction(txn) = e {
            for posting in &txn.postings {
                let res = balances
                    .entry(&posting.account)
                    .or_insert_with(Inventory::new)
                    .add_position(posting);

                let price_entry = if let Some(price) = &posting.price {
                    let mut header = EntryHeader::from_existing(&txn.header);
                    header.add_meta(META_KEY, "from_price");
                    Some(Price {
                        header,
                        currency: posting.units.currency.clone(),
                        amount: price.clone(),
                    })
                } else if let Some(cost) = &posting.cost {
                    if res == BookingResult::REDUCED {
                        None
                    } else {
                        let mut header = EntryHeader::from_existing(&txn.header);
                        header.add_meta(META_KEY, "from_cost");
                        Some(Price {
                            header,
                            currency: posting.units.currency.clone(),
                            amount: Amount::from_cost(cost),
                        })
                    }
                } else {
                    None
                };

                if let Some(p) = price_entry {
                    let key = (
                        p.header.date,
                        p.currency.clone(),
                        p.amount.number,
                        p.amount.currency.clone(),
                    );
                    if !unique_prices.contains(&key) {
                        unique_prices.insert(key);
                        new_prices.push(p.into());
                    }
                }
            }
        }
    }

    (new_prices, Vec::new())
}
