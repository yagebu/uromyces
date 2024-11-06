use crate::errors::UroError;
use crate::ledgers::Ledger;
use crate::types::Entry;
use crate::util::timer::SimpleTimer;

mod balances;
mod documents;
mod implicit_prices;
mod pad;
mod validation;

// A plugin that extends the list of entries (and might emit some errors).
type ExtendPlugin = fn(ledger: &Ledger) -> (Vec<Entry>, Vec<UroError>);

// A validator is a read-only function that might emit some errors.
type Validator = fn(ledger: &Ledger) -> Vec<UroError>;

// The plugins to run before user-specified plugins.
//
// These plugins are independent and can/could be run in parallel.
const PRE_PLUGINS: [(&str, ExtendPlugin); 2] = [
    ("documents", documents::find),
    ("pad", pad::transactions_for_pad_entries),
];

/// Run plugins that should run right after booking.
pub fn run_pre(ledger: &mut Ledger) {
    let mut total = SimpleTimer::new();
    let res = PRE_PLUGINS
        .iter()
        .map(|(name, plugin)| {
            let mut t = SimpleTimer::new();
            let r = plugin(ledger);
            t.log_elapsed(&format!("pre_plugin '{name}'"));
            r
        })
        .collect::<Vec<_>>();
    for (mut entries, mut errors) in res {
        ledger.entries.append(&mut entries);
        ledger.errors.append(&mut errors);
    }
    ledger.entries.sort();
    total.log_elapsed("pre_plugin");
}

const NAMED_PLUGINS: [(&str, ExtendPlugin); 1] =
    [("beancount.plugins.implicit_prices", implicit_prices::add)];

pub fn get_named_plugin(plugin: &str) -> Option<ExtendPlugin> {
    NAMED_PLUGINS.iter().find(|n| n.0 == plugin).map(|n| n.1)
}

/// Run a named plugin.
pub fn run_named_plugin(ledger: &mut Ledger, plugin: &str) -> bool {
    let func = get_named_plugin(plugin);
    let Some(func) = func else { return false };
    let mut t = SimpleTimer::new();
    let (mut entries, mut errors) = func(ledger);
    ledger.entries.append(&mut entries);
    ledger.errors.append(&mut errors);
    ledger.entries.sort();
    t.log_elapsed(&format!("plugin '{plugin}'"));
    true
}

// The validations to run after all other plugins.
const VALIDATORS: [(&str, Validator); 8] = [
    ("account_names", validation::account_names),
    ("open_close", validation::open_close),
    ("duplicate_balances", validation::duplicate_balances),
    ("duplicate_commodities", validation::duplicate_commodities),
    ("active_accounts", validation::active_accounts),
    ("currency_constraints", validation::currency_constraints),
    ("transaction_balances", validation::transaction_balances),
    (
        "check_balance_assertions",
        balances::check_balance_assertions,
    ),
    // All `FilePath`s are absolute, so we do not need to validate this here :)
];

/// Run validations for a ledger and return any validation errors.
///
/// The list of entries is assumed to be sorted.
pub fn run_validations(ledger: &Ledger) -> Vec<UroError> {
    let mut total = SimpleTimer::new();
    let res = VALIDATORS
        .iter()
        .flat_map(|(name, validation)| {
            let mut t = SimpleTimer::new();
            let r = validation(ledger);
            t.log_elapsed(&format!("validation '{name}'"));
            r
        })
        .collect();
    total.log_elapsed("validation");
    res
}
