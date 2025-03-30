//! Options that allow users to change the base accounts for instance.

use std::str::FromStr;

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::display_precision::DisplayPrecisions;
use crate::errors::UroError;
use crate::tolerances::Tolerances;
use crate::types::{Booking, Currency, Decimal, RawDirective, RootAccounts, SummarizationAccounts};

#[derive(Debug)]
pub(crate) enum BeancountOptionError {
    InvalidBookingMethod(String),
    InvalidToleranceDefault(String),
    InvalidToleranceMultiplier(String),
    UnsupportedOption(String),
    UnknownOption(String),
}

impl std::error::Error for BeancountOptionError {}

impl std::fmt::Display for BeancountOptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBookingMethod(s) => {
                write!(f, "Invalid booking method '{s}'")
            }
            Self::InvalidToleranceDefault(s) => {
                write!(f, "Invalid tolerance default '{s}'")
            }
            Self::InvalidToleranceMultiplier(s) => {
                write!(f, "Invalid tolerance multiplier '{s}'")
            }
            Self::UnsupportedOption(s) => {
                write!(f, "The option '{s}' is not (yet) supported in uromyces")
            }
            Self::UnknownOption(s) => {
                write!(f, "Unknown option '{s}'")
            }
        }
    }
}

/// Beancount's options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
#[allow(clippy::module_name_repetitions)]
pub struct BeancountOptions {
    /// Title of the Beancount ledger.
    #[pyo3(get)]
    pub title: String,
    /// The root accounts.
    #[pyo3(get)]
    pub root_accounts: RootAccounts,
    /// Account to accumulate currency conversions for the reporting interval (subaccount of Equity).
    #[pyo3(get)]
    pub account_current_conversions: String,
    /// Account to accumulate currency conversion for the reporting interval (subaccount of Equity).
    #[pyo3(get)]
    pub account_current_earnings: String,
    /// Account to accumulate all previous account balances (subaccount of Equity).
    #[pyo3(get)]
    pub account_previous_balances: String,
    /// Account to accumulate previous currency conversions (subaccount of Equity).
    #[pyo3(get)]
    pub account_previous_conversions: String,
    /// Account that previous Income will be accumulated under (subaccount of Equity).
    #[pyo3(get)]
    pub account_previous_earnings: String,
    /// Wether to render commas.
    #[pyo3(get)]
    pub render_commas: bool,
    /// A list of operating currencies.
    #[pyo3(get)]
    pub operating_currency: Vec<Currency>,
    /// Imaginary currency to convert all units for conversions at a rate of zero.
    #[pyo3(get)]
    conversion_currency: Currency,
    /// A list of document folders.
    #[pyo3(get)]
    pub documents: Vec<String>,
    /// The default booking method to use for accounts that do not specify a booking method.
    #[pyo3(get)]
    pub booking_method: Booking,
    /// The default tolerances per currency.
    pub inferred_tolerance_default: Tolerances,
    /// The default tolerance multiplier.
    pub inferred_tolerance_multiplier: Decimal,
    /// Whether the prepend the directory of the top-level file to sys.path.
    #[pyo3(get)]
    pub insert_pythonpath: bool,
    // not supported:
    // - account_rounding
    // - infer_tolerance_from_cost
    // - plugin_processing_mode
    pub display_precisions: DisplayPrecisions,
}

impl Default for BeancountOptions {
    fn default() -> Self {
        Self {
            title: String::new(),
            root_accounts: RootAccounts::default(),
            account_current_conversions: "Conversions:Current".into(),
            account_current_earnings: "Earnings:Current".into(),
            account_previous_balances: "Opening-Balances".into(),
            account_previous_conversions: "Conversions:Previous".into(),
            account_previous_earnings: "Earnings:Previous".into(),
            render_commas: false,
            operating_currency: Vec::new(),
            conversion_currency: "NOTHING".into(),
            documents: Vec::new(),
            booking_method: Booking::default(),
            inferred_tolerance_default: Tolerances::default(),
            inferred_tolerance_multiplier: Decimal::new(5, 1),
            insert_pythonpath: false,
            display_precisions: DisplayPrecisions::default(),
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
            "title" => value.clone_into(&mut self.title),
            // root accounts
            "name_assets" => self.root_accounts.assets = value.into(),
            "name_liabilities" => self.root_accounts.liabilities = value.into(),
            "name_equity" => self.root_accounts.equity = value.into(),
            "name_income" => self.root_accounts.income = value.into(),
            "name_expenses" => self.root_accounts.expenses = value.into(),
            // other account options
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
                    .map_err(|()| BeancountOptionError::InvalidBookingMethod(value.to_owned()))?;
            }
            "conversion_currency" => {
                self.conversion_currency = value.into();
            }
            // tolerance options
            "inferred_tolerance_default" => self
                .inferred_tolerance_default
                .set_from_option(value)
                .map_err(|()| BeancountOptionError::InvalidToleranceDefault(value.to_owned()))?,
            "inferred_tolerance_multiplier" => {
                self.inferred_tolerance_multiplier = Decimal::from_str(value).map_err(|_| {
                    BeancountOptionError::InvalidToleranceMultiplier(value.to_owned())
                })?;
            }
            "insert_pythonpath" => self.insert_pythonpath = check_boolean_option(value),
            "long_string_maxlines" => {
                // This option is a noop in uromyces as it doesn't handle parsing
                // and the tree-sitter grammar has no such limit.
            }

            "account_rounding" | "infer_tolerance_from_cost" | "plugin_processing_mode" => {
                return Err(BeancountOptionError::UnsupportedOption(key.to_owned()));
            }
            _ => {
                return Err(BeancountOptionError::UnknownOption(key.to_owned()));
            }
        };
        Ok(())
    }

    pub(crate) fn get_summarization_accounts(&self) -> SummarizationAccounts {
        let equity = &self.root_accounts.equity;
        SummarizationAccounts {
            roots: self.root_accounts.clone(),
            current_conversions: equity.join(&self.account_current_conversions),
            current_earnings: equity.join(&self.account_current_earnings),
            previous_balances: equity.join(&self.account_previous_balances),
            previous_conversions: equity.join(&self.account_previous_conversions),
            previous_earnings: equity.join(&self.account_previous_earnings),
        }
    }

    /// Update the option struct with values parsed from a Beancount file.
    pub(crate) fn update_from_raw_directives(
        &mut self,
        directives: &[RawDirective],
    ) -> Vec<UroError> {
        let mut errors = Vec::new();
        for directive in directives {
            if let RawDirective::Option {
                key,
                value,
                filename,
                line,
            } = directive
            {
                let res = self.set_single_option(key, value);
                if let Err(e) = res {
                    errors
                        .push(UroError::new(e.to_string()).with_position(filename.as_ref(), *line));
                }
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_single_option() {
        let mut options = BeancountOptions::default();

        assert!(options.set_single_option("booking_method", "NONE").is_ok());
        assert!(options
            .set_single_option("inferred_tolerance_default", "USD:1.00")
            .is_ok());
    }

    #[test]
    fn test_set_single_option_errors() {
        fn t(o: &str, v: &str, e: &str) {
            let mut options = BeancountOptions::default();
            assert_eq!(options.set_single_option(o, v).unwrap_err().to_string(), e);
        }
        t("booking_method", "asdf", "Invalid booking method 'asdf'");
        t(
            "inferred_tolerance_default",
            "asdf",
            "Invalid tolerance default 'asdf'",
        );
        t(
            "inferred_tolerance_multiplier",
            "1,0",
            "Invalid tolerance multiplier '1,0'",
        );
        t("unknown_option", "asdf", "Unknown option 'unknown_option'");
    }
}
