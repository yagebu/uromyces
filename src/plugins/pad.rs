use hashbrown::{HashMap, HashSet};
use rust_decimal::Decimal;

use crate::Ledger;
use crate::errors::UroError;
use crate::inventory::Inventory;
use crate::tolerances::balance_tolerance;
use crate::types::{Amount, Balance, Currency, Entry, Flag, Pad, Posting, Transaction};

/// This is the state that we need to carry along for each account that we want to pad.
///
/// With a map of per-account `AccountPadder`s, we can iterate over all entries, calling the
/// methods defined below to update the state for each account.
struct AccountPadder<'ledger> {
    ledger: &'ledger Ledger,
    /// The currently active pad entry, i.e., the last seen one.
    active_pad: Option<&'ledger Pad>,
    /// The currencies that were already padded with the currently active pad entry.
    padded_currencies: HashSet<&'ledger Currency>,
    /// The running balance for this account.
    balance: Inventory,
    /// The padding transactions that need to be added to this account.
    new_entries: Vec<Entry>,
}

impl<'ledger> AccountPadder<'ledger> {
    fn new(ledger: &'ledger Ledger) -> Self {
        Self {
            ledger,
            active_pad: None,
            padded_currencies: HashSet::new(),
            balance: Inventory::new(),
            new_entries: Vec::new(),
        }
    }

    fn posting(&mut self, posting: &Posting) {
        self.balance.add_position(&posting.units);
    }

    fn pad(&mut self, entry: &'ledger Pad) {
        self.active_pad = Some(entry);
        self.padded_currencies.clear();
    }

    fn balance(&mut self, entry: &'ledger Balance) {
        let check_amount = &entry.amount;
        let currency = &check_amount.currency;
        let current_balance = self.balance.get(currency, None).unwrap_or(Decimal::ZERO);

        let diff = current_balance - check_amount.number;
        let padded_already = !self.padded_currencies.insert(&check_amount.currency);

        let Some(pad) = &self.active_pad else { return };

        if diff.abs() > balance_tolerance(entry, &self.ledger.options) && !padded_already {
            let diff_units = Amount::new(-diff, currency.clone());
            let txn = Transaction::new(
                pad.header.clone(),
                Flag::PADDING,
                None,
                format!(
                    "(Padding inserted for Balance of {check_amount} for difference {diff_units})"
                ),
                vec![
                    Posting::new_simple(
                        pad.header.filename.clone(),
                        pad.account.clone(),
                        diff_units.clone(),
                    ),
                    Posting::new_simple(
                        pad.header.filename.clone(),
                        pad.source_account.clone(),
                        -diff_units.clone(),
                    ),
                ],
            );
            self.new_entries.push(Entry::Transaction(txn));
            self.balance.add_position(&diff_units);
        }
    }
}

/// Insert transactions for pad entries.
pub fn transactions_for_pad_entries(ledger: &Ledger) -> (Vec<Entry>, Vec<UroError>) {
    let pad_entries = ledger
        .entries
        .iter()
        .filter_map(|e| if let Entry::Pad(p) = e { Some(p) } else { None })
        .collect::<Vec<_>>();

    if pad_entries.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let padded_accounts = pad_entries
        .iter()
        .map(|p| &p.account)
        .collect::<HashSet<_>>();
    let mut account_padders = padded_accounts
        .into_iter()
        .map(|a| (a, AccountPadder::new(ledger)))
        .collect::<HashMap<_, _>>();
    // cache ancestor accounts for which we actually need to call an AccountPadder.
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
                                if account_padders.contains_key(&a) {
                                    active.push(a);
                                }
                            }
                            active
                        });
                    for ancestor in active_ancestors {
                        account_padders
                            .get_mut(ancestor)
                            .expect("account_padders initialised above")
                            .posting(posting);
                    }
                }
            }
            Entry::Pad(e) => {
                let state = account_padders
                    .get_mut(&e.account)
                    .expect("account_padders to exist for Pad above");
                state.pad(e);
            }
            Entry::Balance(e) => {
                if let Some(state) = account_padders.get_mut(&e.account) {
                    state.balance(e);
                }
            }
            _ => {}
        }
    }

    (
        account_padders
            .into_values()
            .flat_map(|s| s.new_entries)
            .collect(),
        Vec::new(),
    )
}
