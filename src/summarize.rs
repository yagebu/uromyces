//! Summarize entries
//!
//! Just like `beancount.ops.summarize`, this module provides functions to summarize entries, e.g.
//! when filtering to a time frame to collect all entries before that time frame to a few entries.

use std::ops::AddAssign;

use hashbrown::{HashMap, HashSet};

use crate::inventory::Inventory;
use crate::inventory::Position;
use crate::types::{
    Account, Date, Entry, EntryHeader, Flag, Posting, SummarizationAccounts, Transaction,
};

type AccountBalances<'a> = HashMap<&'a Account, Inventory>;

/// Accumulate balances by account.
fn balances_by_account(entries: &[Entry]) -> AccountBalances {
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
) -> Vec<Entry> {
    let mut accounts_with_non_empty_balances: Vec<_> =
        balances.iter().filter(|(_, inv)| !inv.is_empty()).collect();
    accounts_with_non_empty_balances.sort_by_key(|(a, _)| *a);
    accounts_with_non_empty_balances
        .into_iter()
        .map(|(account, inv)| {
            let mut postings = Vec::new();
            for pos in inv.iter() {
                postings.push(Posting::new_with_cost(
                    (*account).clone(),
                    pos.units(),
                    pos.cost.clone(),
                ));
                postings.push(Posting::new_with_cost(
                    source_account.clone(),
                    -pos.total_cost(),
                    None,
                ));
            }
            Transaction::new(
                EntryHeader::new(date, None, 0),
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
    );

    // TODO: add latest prices from entries_before
    //       for each currency, cost_currency price pair, keep the last one

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

    clamped_entries.sort();
    clamped_entries
}
