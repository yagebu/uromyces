use hashbrown::{HashMap, HashSet};

use crate::Ledger;
use crate::errors::UroError;
use crate::inventory::{BookingResult, Inventory};
use crate::types::{Amount, Entry, EntryMeta, Price, TagsLinks};

const META_KEY: &str = "__implicit_prices__";

/// Add implicitly defined prices.
pub fn add(ledger: &Ledger) -> (Vec<Entry>, Vec<UroError>) {
    let mut new_prices = Vec::new();

    let mut balances = HashMap::new();
    let mut unique_prices = HashSet::new();

    for transaction in ledger.entries.iter().filter_map(|e| e.as_transaction()) {
        for posting in &transaction.postings {
            let res = balances
                .entry(&posting.account)
                .or_insert_with(Inventory::new)
                .add_position(posting);

            let price_entry = if let Some(price) = &posting.price {
                let mut header = EntryMeta::from_existing(&transaction.meta);
                header.add_meta(META_KEY, "from_price".into());
                Some(Price {
                    date: transaction.date,
                    tags: TagsLinks::default(),
                    links: TagsLinks::default(),
                    meta: header,
                    currency: posting.units.currency.clone(),
                    amount: price.clone(),
                })
            } else if let Some(cost) = &posting.cost {
                if res == BookingResult::REDUCED {
                    None
                } else {
                    let mut header = EntryMeta::from_existing(&transaction.meta);
                    header.add_meta(META_KEY, "from_cost".into());
                    Some(Price {
                        date: transaction.date,
                        tags: TagsLinks::default(),
                        links: TagsLinks::default(),
                        meta: header,
                        currency: posting.units.currency.clone(),
                        amount: Amount::from_cost(cost),
                    })
                }
            } else {
                None
            };

            if let Some(p) = price_entry {
                let key = (
                    p.date,
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

    (new_prices, Vec::new())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::load_string;
    use crate::test_utils::BeancountSnapshot;
    use crate::types::AbsoluteUTF8Path;

    use super::*;

    fn run_implicit_prices_test(path: &Path) {
        let mut snapshot = BeancountSnapshot::load(path);

        let filename: AbsoluteUTF8Path = path.try_into().unwrap();
        let ledger = load_string(snapshot.input(), filename.into());
        let (new_prices, errors) = add(&ledger);

        assert!(errors.is_empty());

        let prices = new_prices
            .iter()
            .filter_map(|e| e.as_price())
            .map(|p| {
                format!(
                    "date={}, currency={}, price={}, meta[\"{}\"]={}",
                    p.date,
                    p.currency,
                    p.amount,
                    META_KEY,
                    p.meta.get(META_KEY).expect("__implicit_prices__ to be set")
                )
            })
            .collect::<Vec<_>>();

        snapshot.add_debug_output("prices", prices);
        snapshot.write();
    }

    #[test]
    fn implicit_prices_test() {
        insta::glob!("implicit_prices_tests/*.beancount", |path| {
            run_implicit_prices_test(path);
        });
    }
}
