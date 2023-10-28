//! Options that allow users to change the base accounts for instance.

use serde::{Deserialize, Serialize};

use crate::tolerances::Tolerances;
use crate::types::{Booking, Currency, Decimal, RawDirective, RootAccounts};

#[derive(Debug)]
pub enum BeancountOptionError {
    InvalidBookingMethod,
    InvalidToleranceDefault,
    UnknownOption(String),
}

impl std::error::Error for BeancountOptionError {}

impl std::fmt::Display for BeancountOptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBookingMethod => {
                write!(f, "Invalid booking method")
            }
            Self::InvalidToleranceDefault => {
                write!(f, "Invalid tolerance default")
            }
            Self::UnknownOption(s) => {
                write!(f, "Unknown option '{s}'")
            }
        }
    }
}

/// Beancount's options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct BeancountOptions {
    /// Title of the Beancount ledger.
    pub title: String,
    /// The root accounts.
    pub root_accounts: RootAccounts,
    /// Account to accumulate currency conversions for the reporting interval (subaccount of Equity).
    pub account_current_conversions: String,
    /// Account to accumulate currency conversion for the reporting interval (subaccount of Equity).
    pub account_current_earnings: String,
    /// Account to accumulate all previous account balances (subaccount of Equity).
    pub account_previous_balances: String,
    /// Account to accumulate previous currency conversions (subaccount of Equity).
    pub account_previous_conversions: String,
    /// Account that previous Income will be accumulated under (subaccount of Equity).
    pub account_previous_earnings: String,
    /// Wether to render commas.
    pub render_commas: bool,
    /// A list of operating currencies.
    pub operating_currency: Vec<Currency>,
    /// Imaginary currency to convert all units for conversions at a rate of zero.
    conversion_currency: Currency,
    /// A list of document folders.
    pub documents: Vec<String>,
    /// The default booking method to use for accounts that do not specify a booking method.
    pub booking_method: Booking,
    // TODO dcontext: DisplayContext
    pub inferred_tolerance_default: Tolerances,
    pub inferred_tolerance_multiplier: Decimal,
    pub infer_tolerance_from_cost: bool,

    insert_pythonpath: bool,
}

impl Default for BeancountOptions {
    fn default() -> Self {
        Self {
            title: String::new(),
            root_accounts: RootAccounts::default(),
            account_current_conversions: "Conversions:Current".into(),
            account_current_earnings: "Earnings:Current".into(),
            account_previous_balances: "Conversions:Previous".into(),
            account_previous_conversions: "Earnings:Previous".into(),
            account_previous_earnings: "Opening-Balances".into(),
            render_commas: false,
            operating_currency: Vec::new(),
            conversion_currency: "NOTHING".into(),
            documents: Vec::new(),
            booking_method: Booking::default(),
            inferred_tolerance_default: Tolerances::default(),
            inferred_tolerance_multiplier: Decimal::new(5, 1),
            infer_tolerance_from_cost: false,
            insert_pythonpath: false,
        }
    }
}

/// Check whether the given option is set to a truthy value.
fn check_boolean_option(val: &str) -> bool {
    let lower = val.to_lowercase();
    lower == "true" || lower == "1" || lower == "yes"
}

impl BeancountOptions {
    /// Set a single Beancount option from a raw key-value pair.
    fn set_single_option(&mut self, key: &str, value: &str) -> Result<(), BeancountOptionError> {
        match key {
            "title" => self.title = value.to_string(),
            // root accounts
            "name_assets" => self.root_accounts.assets = value.into(),
            "name_liabilities" => self.root_accounts.liabilities = value.into(),
            "name_equity" => self.root_accounts.equity = value.into(),
            "name_income" => self.root_accounts.income = value.into(),
            "name_expenses" => self.root_accounts.expenses = value.into(),
            // misc accounts
            "account_current_conversions" => self.account_current_conversions = value.into(),
            "account_current_earnings" => self.account_current_earnings = value.into(),
            "account_previous_balances" => self.account_previous_balances = value.into(),
            "account_previous_conversions" => self.account_previous_conversions = value.into(),
            "account_previous_earnings" => self.account_previous_earnings = value.into(),

            "render_commas" => self.render_commas = check_boolean_option(value),
            "operating_currency" => {
                self.operating_currency.push(value.into());
            }
            "documents" => {
                self.documents.push(value.into());
            }
            "booking_method" => {
                self.booking_method = Booking::try_from(value)
                    .map_err(|()| BeancountOptionError::InvalidBookingMethod)?;
            }
            "account_rounding" => {
                // TODO handle account_rounding
                todo!("account_rounding");
            }
            "conversion_currency" => {
                self.conversion_currency = value.into();
            }
            // tolerance options
            "inferred_tolerance_default" => {
                self.inferred_tolerance_default.set_from_option(value)?;
            }
            "inferred_tolerance_multiplier" => {
                todo!("inferred_tolerance_multiplier {}", value);
            }
            "infer_tolerance_from_cost" => {
                todo!("infer_tolerance_from_cost {}", value);
            }
            "plugin_processing_mode" => {
                todo!("plugin_processing_mode {}", value);
            }
            "insert_pythonpath" => self.insert_pythonpath = check_boolean_option(value),
            "long_string_maxlines" => {
                // This option is a noop in uromyces as it doesn't handle parsing
                // and the tree-sitter grammar has no such limit.
            }
            _ => {
                return Err(BeancountOptionError::UnknownOption(key.to_string()));
            }
        };
        Ok(())
    }

    /// Update the option struct with values parsed from a Beancount file.
    pub(crate) fn update_from_raw_directives(&mut self, directives: &[RawDirective]) {
        for directive in directives {
            if let RawDirective::Option { key, value, .. } = directive {
                // TODO emit error
                self.set_single_option(key, value).unwrap();
            }
        }
    }
}
