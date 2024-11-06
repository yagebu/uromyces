use hashbrown::{HashMap, HashSet};
use rust_decimal::Decimal;

use crate::errors::UroError;
use crate::inventory::Inventory;
use crate::tolerances::balance_tolerance;
use crate::types::{Account, Balance, Entry, Posting};
use crate::Ledger;

/// A balance assertion failed.
struct BalanceCheckError<'a>(&'a Account, &'a Balance, Decimal);

impl From<BalanceCheckError<'_>> for crate::errors::UroError {
    fn from(e: BalanceCheckError) -> Self {
        let BalanceCheckError(account, balance_entry, diff_amount) = &e;
        let diff_msg = if *diff_amount > Decimal::ZERO {
            format!("{diff_amount} too much")
        } else {
            format!("{} too little", -diff_amount)
        };
        let expected_amount = &balance_entry.amount;
        let currency = &expected_amount.currency;
        let balance = expected_amount.number + diff_amount;
        let msg = format!(
                "Balance failed for '{account}': expected {expected_amount} != accumulated {balance} {currency} ({diff_msg})"
            );
        Self::new(msg).with_entry(*balance_entry)
    }
}

/// The state we need for each account that we want to check balances for.
///
/// With a map of these per account, we can iterate over all entries, calling the methods defined
/// belows to update the state along the way.
struct BalanceChecker<'ledger> {
    ledger: &'ledger Ledger,
    balance: Inventory,
    errors: Vec<UroError>,
}

impl<'ledger> BalanceChecker<'ledger> {
    fn new(ledger: &'ledger Ledger) -> Self {
        Self {
            ledger,
            balance: Inventory::new(),
            errors: Vec::new(),
        }
    }

    fn posting(&mut self, posting: &Posting) {
        // we only add the units here, we do not care about the cost for balance assertions.
        self.balance.add_position(&posting.units);
    }

    fn balance(&mut self, entry: &'ledger Balance) {
        let account = &entry.account;
        let expected_amount = &entry.amount;
        let current_balance = self
            .balance
            .get(expected_amount.currency.clone(), None)
            .unwrap_or(Decimal::ZERO);

        let diff = current_balance - expected_amount.number;
        let diff_abs = diff.abs();

        if diff_abs > balance_tolerance(entry, &self.ledger.options) {
            self.errors
                .push(BalanceCheckError(account, entry, diff).into());
        }
    }
}

/// Check balance assertions.
pub fn check_balance_assertions(ledger: &Ledger) -> Vec<UroError> {
    let balance_entries = ledger
        .entries
        .iter()
        .filter_map(|e| {
            if let Entry::Balance(b) = e {
                Some(b)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if balance_entries.is_empty() {
        return Vec::new();
    }

    let checked_accounts = balance_entries
        .iter()
        .map(|p| &p.account)
        .collect::<HashSet<_>>();
    let mut balance_checkers = checked_accounts
        .into_iter()
        .map(|a| (a, BalanceChecker::new(ledger)))
        .collect::<HashMap<_, _>>();
    let mut active_ancestors_by_account = HashMap::new();

    for entry in &ledger.entries {
        match entry {
            Entry::Transaction(e) => {
                for posting in &e.postings {
                    let active_ancestors = active_ancestors_by_account
                        .entry(&posting.account)
                        .or_insert_with(|| {
                            let mut active = Vec::new();
                            let mut account = Some(posting.account.clone());
                            while let Some(a) = account {
                                account = a.parent();
                                if balance_checkers.contains_key(&a) {
                                    active.push(a);
                                }
                            }
                            active
                        });
                    for ancestor in active_ancestors {
                        balance_checkers
                            .get_mut(ancestor)
                            .expect("balance_checker to be created above")
                            .posting(posting);
                    }
                }
            }
            Entry::Balance(e) => {
                let state = balance_checkers
                    .get_mut(&e.account)
                    .expect("balance_checker to be created above");
                state.balance(e);
            }
            _ => {}
        }
    }

    let mut sorted_checkers = balance_checkers.into_iter().collect::<Vec<_>>();
    sorted_checkers.sort_unstable_by_key(|v| v.0);
    sorted_checkers
        .into_iter()
        .flat_map(|s| s.1.errors)
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use crate::{booking, ledgers::RawLedger, parse};

    use super::*;

    fn check(s: &str) -> Vec<String> {
        let res = parse::parse_string(s, &None);
        let raw_ledger = RawLedger::from_single_parsed_file("/test/path".try_into().unwrap(), res);
        let ledger = booking::book_entries(raw_ledger);
        check_balance_assertions(&ledger)
            .into_iter()
            .map(|e| e.message().into())
            .collect()
    }

    #[test]
    fn test_simple_error() {
        insta::assert_json_snapshot!(check(r"
2013-05-01 open Assets:US:Checking

2013-05-03 balance Assets:US:Checking   100 USD
"), @r###"
        [
          "Balance failed for 'Assets:US:Checking': expected 100 USD != accumulated 0 USD (100 too little)"
        ]
        "###);
    }

    #[test]
    fn test_parents() {
        insta::assert_json_snapshot!(check(r"
2013-05-01 open Assets:Bank
2013-05-01 open Assets:Bank:Checking1
2013-05-01 open Assets:Bank:Checking2
2013-05-01 open Assets:Bank:Savings   ;; Will go negative
2013-05-01 open Equity:Opening-Balances

2013-05-02 *
  Assets:Bank:Checking1                100 USD
  Equity:Opening-Balances

2013-05-03 *
  Assets:Bank:Checking2                10 USD
  Equity:Opening-Balances

2013-05-04 *
  Assets:Bank:Savings                 -50 USD
  Equity:Opening-Balances

2013-05-05 balance Assets:Bank:Checking1  100 USD
2013-05-05 balance Assets:Bank:Checking2   10 USD
2013-05-05 balance Assets:Bank:Savings    -50 USD
2013-05-05 balance Assets:Bank             60 USD
"), @"[]");
    }

    #[test]
    fn test_precision() {
        insta::assert_json_snapshot!(check(r"
2013-05-01 open Assets:Bank:Checking
2013-05-01 open Income:Interest

2013-05-02 *
  Assets:Bank:Checking        0.00001 USD
  Income:Interest

2013-05-03 balance Assets:Bank:Checking   0.00 USD

2013-05-03 *
  Assets:Bank:Checking        0.00001 USD
  Income:Interest

2013-05-04 balance Assets:Bank:Checking   0.00 USD

2013-05-04 *
  Assets:Bank:Checking        0.015 USD
  Income:Interest

2013-05-05 balance Assets:Bank:Checking   0.01502 USD
"), @"[]");
    }

    #[test]
    fn test_mixed_cost_and_no_cost() {
        insta::assert_json_snapshot!(check(r"
2013-05-01 open Assets:Invest
2013-05-01 open Equity:Opening-Balances

2013-05-01 *
  Assets:Invest                100 HOOL {14.33 USD}
  Equity:Opening-Balances

2013-05-02 *
  Assets:Invest               -100 HOOL @ 15.66 USD
  Equity:Opening-Balances

2013-05-10 balance Assets:Invest   0 HOOL
"), @"[]");
    }

    #[test]
    fn test_balance_with_tolerance() {
        insta::assert_json_snapshot!(check(r"
2013-05-01 open Assets:Bank:Checking
2013-05-01 open Equity:Opening-Balances

2013-05-03 *
  Assets:Bank:Checking              23.024 USD
  Equity:Opening-Balances

2015-05-02 balance Assets:Bank:Checking   23.022 ~ 0.001 USD
2015-05-03 balance Assets:Bank:Checking   23.023 ~ 0.001 USD
2015-05-04 balance Assets:Bank:Checking   23.024 ~ 0.001 USD
2015-05-05 balance Assets:Bank:Checking   23.025 ~ 0.001 USD
2015-05-06 balance Assets:Bank:Checking   23.026 ~ 0.001 USD

2015-05-10 balance Assets:Bank:Checking   23.03 ~ 0.01 USD
"), @r###"
        [
          "Balance failed for 'Assets:Bank:Checking': expected 23.022 USD != accumulated 23.024 USD (0.002 too much)",
          "Balance failed for 'Assets:Bank:Checking': expected 23.026 USD != accumulated 23.024 USD (0.002 too little)"
        ]
        "###);
    }
}
