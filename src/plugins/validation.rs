use hashbrown::{HashMap, HashSet};

use crate::booking;
use crate::errors::UroError;
use crate::tolerances::Tolerances;
use crate::types::{Account, Balance, Currency, Date, Entry};
use crate::Ledger;

struct InvalidAccountName(Account);
impl InvalidAccountName {
    fn new(a: &Account) -> Self {
        Self(a.clone())
    }
}
impl From<InvalidAccountName> for UroError {
    fn from(val: InvalidAccountName) -> Self {
        UroError::new(format!(
            "Invalid account name '{}' (invalid root account).",
            val.0
        ))
    }
}

/// Check that:
///
/// - Each account name starts with one of the root accounts.
/// - Each account name consists of the allowed characters (for simplicity, the lexer+parser allow
///   all non-ASCII Unicode (TODO)
pub fn account_names(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();

    let all_accounts = ledger
        .entries
        .iter()
        .flat_map(Entry::get_accounts)
        .collect::<HashSet<_>>();
    let roots = &ledger.options.root_accounts;

    for account in all_accounts {
        if !account.has_valid_root(roots) {
            errors.push(InvalidAccountName::new(account).into());
        }
        // TODO: check full account syntax
    }

    errors
}

/// Check that:
///
/// - Each account is opened at most once and closed at most once.
/// - Only open accounts are closed.
pub fn open_close(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut open_accounts = HashSet::new();
    let mut closed_accounts = HashSet::new();

    for entry in &ledger.entries {
        match entry {
            Entry::Open(e) => {
                if open_accounts.contains(&e.account) {
                    errors.push(
                        UroError::new(format!(
                            "Duplicate open directive for account {}.",
                            e.account
                        ))
                        .with_entry(e),
                    );
                } else {
                    open_accounts.insert(&e.account);
                }
            }
            Entry::Close(e) => {
                if closed_accounts.contains(&e.account) {
                    errors.push(
                        UroError::new(format!(
                            "Duplicate close directive for account {}.",
                            e.account
                        ))
                        .with_entry(e),
                    );
                } else {
                    if !open_accounts.contains(&e.account) {
                        errors.push(
                            UroError::new(format!("Closing unopened account {}.", e.account))
                                .with_entry(e),
                        );
                    };
                    closed_accounts.insert(&e.account);
                }
            }
            _ => {}
        }
    }
    errors
}

/// Check that:
///
/// - No duplicate balances (same account, date and currency) exist with different amounts.
pub fn duplicate_balances(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut balances: HashMap<(&Account, &Date, &Currency), &Balance> = HashMap::new();

    for entry in &ledger.entries {
        if let Entry::Balance(e) = entry {
            let key = (&e.account, &e.header.date, &e.amount.currency);
            match balances.get(&key) {
                Some(b) => {
                    if b.amount != e.amount {
                        errors.push(
                            UroError::new("Duplicate balance assertions with different amounts.")
                                .with_entry(e),
                        );
                    }
                }
                None => {
                    balances.insert(key, e);
                }
            };
        }
    }
    errors
}

/// Check that:
///
/// - No duplicate commodities exist.
pub fn duplicate_commodities(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut commodities = HashSet::new();

    for entry in &ledger.entries {
        if let Entry::Commodity(e) = entry {
            if !commodities.insert(&e.currency) {
                errors.push(
                    UroError::new(format!("Duplicate commodity directive for {}.", e.currency))
                        .with_entry(e),
                );
            }
        }
    }
    errors
}

/// Check that:
///
/// - Only active (opened) accounts are used. Notes, balances and documents are allowed to occur
///   after the closing date.
pub fn active_accounts(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut currently_open_accounts = HashSet::new();
    let mut opened_accounts = HashSet::new();

    let mut errs = Vec::new();

    for entry in &ledger.entries {
        match entry {
            Entry::Open(e) => {
                currently_open_accounts.insert(&e.account);
                opened_accounts.insert(&e.account);
            }
            Entry::Close(e) => {
                currently_open_accounts.remove(&e.account);
            }
            _ => {
                for account in entry.get_accounts() {
                    if !(currently_open_accounts.contains(account)
                        || opened_accounts.contains(account)
                            && matches!(
                                entry,
                                Entry::Document(..) | Entry::Note(..) | Entry::Balance(..)
                            ))
                    {
                        errs.push((account, entry));
                    }
                }
            }
        }
    }

    for (account, entry) in errs {
        let message = if opened_accounts.contains(account) {
            format!("Invalid reference to inactive account {account}.")
        } else {
            format!("Invalid reference to unknown account {account}.")
        };
        errors.push(UroError::new(message).with_entry(entry));
    }
    errors
}

/// Check that:
///
/// - All transactions balance.
pub fn transaction_balances(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    for entry in &ledger.entries {
        if let Entry::Transaction(e) = entry {
            let residual = booking::compute_residual(&e.postings);
            let tolerances = Tolerances::infer_from_booked(&e.postings, &ledger.options);
            if !tolerances.is_small(&residual) {
                errors.push(UroError::new("Transaction does not balance").with_entry(e));
            }
        }
    }
    errors
}

/// Check that:
///
/// - For accounts that declare a list of currencies, only these currencies are used in
///   transactions and balances.
pub fn currency_constraints(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut currency_constraints: HashMap<&Account, &Vec<Currency>> = HashMap::new();

    for entry in &ledger.entries {
        if let Entry::Open(e) = entry {
            if !e.currencies.is_empty() {
                currency_constraints.insert(&e.account, &e.currencies);
            }
        }
    }

    for entry in &ledger.entries {
        match entry {
            Entry::Transaction(e) => {
                for posting in &e.postings {
                    let account = &posting.account;
                    if let Some(constraints) = currency_constraints.get(account) {
                        let currency = &posting.units.currency;
                        if !constraints.contains(currency) {
                            errors.push(
                                UroError::new(format!(
                                    "Invalid currency '{currency}' for account '{account}'"
                                ))
                                .with_entry(e),
                            );
                        }
                    }
                }
            }
            Entry::Balance(e) => {
                let account = &e.account;
                if let Some(constraints) = currency_constraints.get(account) {
                    let currency = &e.amount.currency;
                    if !constraints.contains(currency) {
                        errors.push(
                            UroError::new(format!(
                                "Invalid currency '{currency}' for account '{account}'"
                            ))
                            .with_entry(e),
                        );
                    }
                }
            }
            _ => (),
        }
    }
    errors
}
