//! Summarize entries
//!
//! Just like `beancount.ops.summarize`, this module provides functions to summarize entries, e.g.
//! when filtering to a time frame to collect all entries before that time frame to a few entries.

use std::ops::AddAssign;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;

use crate::inventory::Inventory;
use crate::inventory::Position;
use crate::types::Filename;
use crate::types::{
    Account, Date, Entry, EntryHeader, Flag, Posting, SummarizationAccounts, Transaction,
};

type AccountBalances<'a> = HashMap<&'a Account, Inventory>;

/// Accumulate balances by account.
fn balances_by_account(entries: &[Entry]) -> AccountBalances<'_> {
    let mut balances = HashMap::new();
    for e in entries {
        if let Entry::Transaction(txn) = e {
            for pos in &txn.postings {
                balances
                    .entry(&pos.account)
                    .or_insert_with(Inventory::new)
                    .add_position(pos);
            }
        }
    }
    balances
}

/// Create summarisation entries.
///
/// For the given balances, create entries at the given date. These entries will be sorted by the
/// account name
fn create_summarisation_entries(
    balances: &AccountBalances,
    date: Date,
    source_account: &Account,
    accounts: &SummarizationAccounts,
) -> Vec<Entry> {
    let summarize_filename = Filename::new_dummy("summarize");
    let mut accounts_with_non_empty_balances: Vec<_> = balances
        .iter()
        .filter(|(a, _)| !accounts.roots.is_income_statement_account(a))
        .filter(|(_, inv)| !inv.is_empty())
        .collect();
    accounts_with_non_empty_balances.sort_by_key(|(a, _)| *a);
    accounts_with_non_empty_balances
        .into_iter()
        .map(|(account, inv)| {
            let mut postings = Vec::new();
            for pos in inv.iter() {
                postings.push(Posting::new_with_cost(
                    summarize_filename.clone(),
                    (*account).clone(),
                    pos.units(),
                    pos.cost.clone(),
                ));
                postings.push(Posting::new_with_cost(
                    summarize_filename.clone(),
                    source_account.clone(),
                    -pos.total_cost(),
                    None,
                ));
            }
            Transaction::new(
                EntryHeader::new(date, summarize_filename.clone(), 0),
                Flag::SUMMARIZE,
                None,
                format!("Opening balance for '{account}' (Summarization)"),
                postings,
            )
            .into()
        })
        .collect()
}

/// Limit entries to a given time interval.
///
/// We first accumulate balances previous to `begin_date`. Of those, we can move the balances of
/// income statement accounts (income, expenses) "previous earnings" account. And then summarize
/// the remaining balances (which now include additional previous earnings). Beancount internally
/// does this in two separate steps (using the `transfer_balances` function for the first step), however
/// we can do it in one step.
///
/// For the summarization, we also want to
/// - keep latest prices entries from before `begin_date`
/// - keep all open entries from before `begin_date`
/// - filter out any income/expense balance assertion since those would now fail
///   (at least if they were added to previous earnings)
#[must_use]
pub fn clamp(
    entries: &[Entry],
    begin_date: Date,
    end_date: Date,
    accounts: &SummarizationAccounts,
) -> Vec<Entry> {
    debug_assert!(entries.is_sorted());
    let start_index = entries.partition_point(|e| e.get_header().date < begin_date);
    let end_index = entries.partition_point(|e| e.get_header().date < end_date);
    let entries_before = &entries[0..start_index];
    let entries_during = &entries[start_index..end_index];

    let mut balances_before = balances_by_account(entries_before);

    // Get the income statement accounts that need to be transferred and accumulate the previous earnings.
    let mut transfered_income_statement_accounts = HashSet::new();
    let mut previous_earnings_balance = Inventory::new();
    for (account, inv) in &balances_before {
        if accounts.roots.is_income_statement_account(account) {
            transfered_income_statement_accounts.insert(*account);
            previous_earnings_balance += inv;
        }
    }
    balances_before
        .entry(&accounts.previous_earnings)
        .or_default()
        .add_assign(&previous_earnings_balance);

    // Create summarisation entries
    let summarisation_entry_date = begin_date.previous_day().unwrap_or(begin_date);
    let mut clamped_entries = create_summarisation_entries(
        &balances_before,
        summarisation_entry_date,
        &accounts.previous_balances,
        accounts,
    );

    // for each currency, cost_currency price pair, keep the last one
    clamped_entries.extend(
        entries_before
            .iter()
            .filter_map(|e| {
                if let Entry::Price(p) = e {
                    Some(((&p.currency, &p.amount.currency), e))
                } else {
                    None
                }
            })
            .collect::<IndexMap<_, _>>()
            .into_values()
            .cloned(),
    );

    // Add all open entries from `entries_before`
    clamped_entries.extend(
        entries_before
            .iter()
            .filter(|e| matches!(e, Entry::Open(..)))
            .cloned(),
    );

    // Add all entries in the time interval, except for Balance entries of income statement
    // accounts that we transfered to the previous earnings account.
    clamped_entries.extend(
        entries_during
            .iter()
            .filter(|b| {
                if let Entry::Balance(b) = b {
                    !transfered_income_statement_accounts.contains(&b.account)
                } else {
                    true
                }
            })
            .cloned(),
    );

    // TODO: conversions
    //       we want to sum up all postings, and insert an entry with negative
    //       positions for the costs

    // debug_assert!(clamped_entries.is_sorted());
    clamped_entries.sort();
    clamped_entries
}

#[cfg(test)]
mod tests {
    use crate::load_string;

    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_summarize_clamp() {
        // From beancount.ops.summarize_test.TestClamp
        let input = r#"
2012-01-01 open Income:Salary
2012-01-01 open Expenses:Taxes
2012-01-01 open Assets:US:Checking
2012-01-01 open Assets:CA:Checking

2012-03-01 * "Some income and expense to be summarized"
  Income:Salary        10000.00 USD
  Expenses:Taxes        3600.00 USD
  Assets:US:Checking  -13600.00 USD

2012-03-02 * "Some conversion to be summarized"
  Assets:US:Checking   -5000.00 USD @ 1.2 CAD
  Assets:CA:Checking    6000.00 CAD

;; 2012-06-01  BEGIN --------------------------------

2012-08-01 * "Some income and expense to show"
  Income:Salary        11000.00 USD
  Expenses:Taxes        3200.00 USD
  Assets:US:Checking  -14200.00 USD

2012-08-02 * "Some other conversion to be summarized"
  Assets:US:Checking   -3000.00 USD @ 1.25 CAD
  Assets:CA:Checking    3750.00 CAD

;; 2012-09-01  END   --------------------------------

2012-11-01 * "Some income and expense to be truncated"
  Income:Salary        10000.00 USD
  Expenses:Taxes        3600.00 USD
  Assets:US:Checking  -13600.00 USD
"#;

        let ledger = load_string(input, "<string>".try_into().unwrap());
        let clamped_entries = clamp(
            &ledger.entries,
            Date::from_ymd_opt(2012, 6, 1).unwrap(),
            Date::from_ymd_opt(2012, 9, 1).unwrap(),
            &ledger.options.get_summarization_accounts(),
        );
        insta::assert_json_snapshot!(clamped_entries, @r#"
        [
          {
            "t": "Open",
            "date": "2012-01-01",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 2,
            "account": "Income:Salary",
            "currencies": [],
            "booking": null
          },
          {
            "t": "Open",
            "date": "2012-01-01",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 3,
            "account": "Expenses:Taxes",
            "currencies": [],
            "booking": null
          },
          {
            "t": "Open",
            "date": "2012-01-01",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 4,
            "account": "Assets:US:Checking",
            "currencies": [],
            "booking": null
          },
          {
            "t": "Open",
            "date": "2012-01-01",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 5,
            "account": "Assets:CA:Checking",
            "currencies": [],
            "booking": null
          },
          {
            "t": "Transaction",
            "date": "2012-05-31",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<summarize>",
            "line": 0,
            "flag": "S",
            "payee": null,
            "narration": "Opening balance for 'Assets:CA:Checking' (Summarization)",
            "postings": [
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Assets:CA:Checking",
                "units": {
                  "number": "6000.00",
                  "currency": "CAD"
                },
                "price": null,
                "cost": null,
                "flag": null
              },
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Equity:Opening-Balances",
                "units": {
                  "number": "-6000.00",
                  "currency": "CAD"
                },
                "price": null,
                "cost": null,
                "flag": null
              }
            ]
          },
          {
            "t": "Transaction",
            "date": "2012-05-31",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<summarize>",
            "line": 0,
            "flag": "S",
            "payee": null,
            "narration": "Opening balance for 'Assets:US:Checking' (Summarization)",
            "postings": [
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Assets:US:Checking",
                "units": {
                  "number": "-18600.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              },
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Equity:Opening-Balances",
                "units": {
                  "number": "18600.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              }
            ]
          },
          {
            "t": "Transaction",
            "date": "2012-05-31",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<summarize>",
            "line": 0,
            "flag": "S",
            "payee": null,
            "narration": "Opening balance for 'Equity:Earnings:Previous' (Summarization)",
            "postings": [
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Equity:Earnings:Previous",
                "units": {
                  "number": "13600.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              },
              {
                "filename": "<summarize>",
                "line": null,
                "meta": [],
                "account": "Equity:Opening-Balances",
                "units": {
                  "number": "-13600.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              }
            ]
          },
          {
            "t": "Transaction",
            "date": "2012-08-01",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 18,
            "flag": "*",
            "payee": null,
            "narration": "Some income and expense to show",
            "postings": [
              {
                "filename": "<string>",
                "line": 19,
                "meta": [],
                "account": "Income:Salary",
                "units": {
                  "number": "11000.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              },
              {
                "filename": "<string>",
                "line": 20,
                "meta": [],
                "account": "Expenses:Taxes",
                "units": {
                  "number": "3200.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              },
              {
                "filename": "<string>",
                "line": 21,
                "meta": [],
                "account": "Assets:US:Checking",
                "units": {
                  "number": "-14200.00",
                  "currency": "USD"
                },
                "price": null,
                "cost": null,
                "flag": null
              }
            ]
          },
          {
            "t": "Transaction",
            "date": "2012-08-02",
            "meta": [],
            "tags": [],
            "links": [],
            "filename": "<string>",
            "line": 23,
            "flag": "*",
            "payee": null,
            "narration": "Some other conversion to be summarized",
            "postings": [
              {
                "filename": "<string>",
                "line": 24,
                "meta": [],
                "account": "Assets:US:Checking",
                "units": {
                  "number": "-3000.00",
                  "currency": "USD"
                },
                "price": {
                  "number": "1.25",
                  "currency": "CAD"
                },
                "cost": null,
                "flag": null
              },
              {
                "filename": "<string>",
                "line": 25,
                "meta": [],
                "account": "Assets:CA:Checking",
                "units": {
                  "number": "3750.00",
                  "currency": "CAD"
                },
                "price": null,
                "cost": null,
                "flag": null
              }
            ]
          }
        ]
        "#);
    }
}
