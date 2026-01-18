use hashbrown::{HashMap, HashSet};

use crate::Ledger;
use crate::conversions::get_weight;
use crate::errors::UroError;
use crate::inventory::Inventory;
use crate::tolerances::Tolerances;
use crate::types::{
    Account, Balance, Close, Commodity, Currency, Date, Document, Entry, Open, Transaction,
};

struct InvalidAccountNameRoot<'a>(&'a Account);
impl From<InvalidAccountNameRoot<'_>> for UroError {
    fn from(val: InvalidAccountNameRoot) -> Self {
        UroError::new(format!(
            "Invalid account name '{}' (invalid root account).",
            val.0
        ))
    }
}

struct InvalidAccountNameSyntax<'a>(&'a Account);
impl From<InvalidAccountNameSyntax<'_>> for UroError {
    fn from(val: InvalidAccountNameSyntax) -> Self {
        UroError::new(format!(
            "Invalid account name '{}' (does not match valid pattern).",
            val.0
        ))
    }
}

/// Check that:
///
/// - Each account name starts with one of the root accounts.
/// - Each account name matches the valid pattern (uppercase/digit start, letters/digits/hyphens).
pub fn account_names(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();

    let all_accounts = ledger
        .entries
        .iter()
        .flat_map(Entry::accounts)
        .collect::<HashSet<_>>();
    let roots = &ledger.options.root_accounts;

    for account in all_accounts {
        if !account.has_valid_root(roots) {
            errors.push(InvalidAccountNameRoot(account).into());
        } else if !account.has_valid_name() {
            errors.push(InvalidAccountNameSyntax(account).into());
        }
    }

    errors
}

struct DuplicateOpenDirective<'a>(&'a Open);
impl From<DuplicateOpenDirective<'_>> for UroError {
    fn from(val: DuplicateOpenDirective) -> Self {
        UroError::new(format!(
            "Duplicate open directive for account {}.",
            val.0.account
        ))
        .with_entry(val.0)
    }
}

struct DuplicateCloseDirective<'a>(&'a Close);
impl From<DuplicateCloseDirective<'_>> for UroError {
    fn from(val: DuplicateCloseDirective) -> Self {
        UroError::new(format!(
            "Duplicate close directive for account {}.",
            val.0.account
        ))
        .with_entry(val.0)
    }
}

struct ClosingUnopenedAccount<'a>(&'a Close);
impl From<ClosingUnopenedAccount<'_>> for UroError {
    fn from(val: ClosingUnopenedAccount) -> Self {
        UroError::new(format!("Closing unopened account {}.", val.0.account)).with_entry(val.0)
    }
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
                    errors.push(DuplicateOpenDirective(e).into());
                } else {
                    open_accounts.insert(&e.account);
                }
            }
            Entry::Close(e) => {
                if closed_accounts.contains(&e.account) {
                    errors.push(DuplicateCloseDirective(e).into());
                } else {
                    if !open_accounts.contains(&e.account) {
                        errors.push(ClosingUnopenedAccount(e).into());
                    }
                    closed_accounts.insert(&e.account);
                }
            }
            _ => {}
        }
    }
    errors
}

struct DuplicateDifferingBalanceDirective<'a>(&'a Balance);
impl From<DuplicateDifferingBalanceDirective<'_>> for UroError {
    fn from(val: DuplicateDifferingBalanceDirective) -> Self {
        UroError::new("Duplicate balance assertions with different amounts.").with_entry(val.0)
    }
}

/// Check that:
///
/// - No duplicate balances (same account, date and currency) exist with different amounts.
pub fn duplicate_balances(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut balances: HashMap<(&Account, &Date, &Currency), &Balance> = HashMap::new();

    for balance in ledger.entries.iter().filter_map(|e| e.as_balance()) {
        let key = (&balance.account, &balance.date, &balance.amount.currency);
        match balances.get(&key) {
            Some(b) => {
                if b.amount != balance.amount {
                    errors.push(DuplicateDifferingBalanceDirective(balance).into());
                }
            }
            None => {
                balances.insert(key, balance);
            }
        }
    }
    errors
}

struct DuplicateCommodityDirective<'a>(&'a Commodity);
impl From<DuplicateCommodityDirective<'_>> for UroError {
    fn from(val: DuplicateCommodityDirective) -> Self {
        UroError::new(format!(
            "Duplicate commodity directive for {}.",
            val.0.currency
        ))
        .with_entry(val.0)
    }
}

/// Check that:
///
/// - No duplicate commodities exist.
pub fn duplicate_commodities(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut commodities = HashSet::new();

    for entry in &ledger.entries {
        if let Entry::Commodity(e) = entry
            && !commodities.insert(&e.currency)
        {
            errors.push(DuplicateCommodityDirective(e).into());
        }
    }
    errors
}

struct InvalidReferenceToInactiveAccount<'a>(&'a Account, &'a Entry);
impl From<InvalidReferenceToInactiveAccount<'_>> for UroError {
    fn from(val: InvalidReferenceToInactiveAccount) -> Self {
        UroError::new(format!("Invalid reference to inactive account {}.", val.0)).with_entry(val.1)
    }
}

struct InvalidReferenceToUnknownAccount<'a>(&'a Account, &'a Entry);
impl From<InvalidReferenceToUnknownAccount<'_>> for UroError {
    fn from(val: InvalidReferenceToUnknownAccount) -> Self {
        UroError::new(format!("Invalid reference to unknown account {}.", val.0)).with_entry(val.1)
    }
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
                for account in entry.accounts() {
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
        errors.push(if opened_accounts.contains(account) {
            InvalidReferenceToInactiveAccount(account, entry).into()
        } else {
            InvalidReferenceToUnknownAccount(account, entry).into()
        });
    }
    errors
}

struct TransactionDoesNotBalance<'a>(&'a Transaction);
impl From<TransactionDoesNotBalance<'_>> for UroError {
    fn from(val: TransactionDoesNotBalance) -> Self {
        UroError::new("Transaction does not balance").with_entry(val.0)
    }
}

/// Check that:
///
/// - All transactions balance.
pub fn transaction_balances(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();

    for transaction in ledger.entries.iter().filter_map(|e| e.as_transaction()) {
        let residual = transaction
            .postings
            .iter()
            .map(get_weight)
            .collect::<Inventory>();
        let tolerances = Tolerances::infer_from_booked(&transaction.postings, &ledger.options);
        if !tolerances.is_small(&residual) {
            errors.push(TransactionDoesNotBalance(transaction).into());
        }
    }

    errors
}

struct InvalidCurrencyInTransaction<'a>(&'a Currency, &'a Account, &'a Transaction);
impl From<InvalidCurrencyInTransaction<'_>> for UroError {
    fn from(val: InvalidCurrencyInTransaction) -> Self {
        UroError::new(format!(
            "Invalid currency '{0}' for account '{1}'",
            val.0, val.1
        ))
        .with_entry(val.2)
    }
}

struct InvalidCurrencyInBalance<'a>(&'a Currency, &'a Account, &'a Balance);
impl From<InvalidCurrencyInBalance<'_>> for UroError {
    fn from(val: InvalidCurrencyInBalance) -> Self {
        UroError::new(format!(
            "Invalid currency '{0}' for account '{1}'",
            val.0, val.1
        ))
        .with_entry(val.2)
    }
}

/// Check that:
///
/// - For accounts that declare a list of currencies, only these currencies are used in
///   transactions and balances.
pub fn currency_constraints(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();
    let mut currency_constraints: HashMap<&Account, &Vec<Currency>> = HashMap::new();

    for entry in &ledger.entries {
        if let Entry::Open(e) = entry
            && !e.currencies.is_empty()
        {
            currency_constraints.insert(&e.account, &e.currencies);
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
                            errors.push(InvalidCurrencyInTransaction(currency, account, e).into());
                        }
                    }
                }
            }
            Entry::Balance(e) => {
                let account = &e.account;
                if let Some(constraints) = currency_constraints.get(account) {
                    let currency = &e.amount.currency;
                    if !constraints.contains(currency) {
                        errors.push(InvalidCurrencyInBalance(currency, account, e).into());
                    }
                }
            }
            _ => (),
        }
    }
    errors
}

struct DocumentFileDoesNotExist<'a>(&'a Document);
impl From<DocumentFileDoesNotExist<'_>> for UroError {
    fn from(val: DocumentFileDoesNotExist) -> Self {
        UroError::new(format!("File does not exist: '{}'", val.0.filename)).with_entry(val.0)
    }
}

/// Check that:
///
/// - All document files exist.
pub fn document_files_exist(ledger: &Ledger) -> Vec<UroError> {
    let mut errors = Vec::new();

    for document in ledger.entries.iter().filter_map(|e| e.as_document()) {
        if !document.filename.as_ref().exists() {
            errors.push(DocumentFileDoesNotExist(document).into());
        }
    }

    errors
}
