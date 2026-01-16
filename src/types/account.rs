use std::fmt::{Debug, Display};
use std::sync::LazyLock;

use pyo3::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::interned_string::InternedString;

/// Components of the account are separated by colons.
const SEPARATOR: char = ':';

/// An account name.
///
/// An account name is a string where components of the account are separated by `:`. The account
/// name needs to start with one of the five root accounts. There are some further restrictions on
/// the syntax that is ensured by the parser.
///
/// To speed up common operations on account names and reduce memory usage, this uses a string
/// interner.
#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromPyObject,
    IntoPyObjectRef,
)]
pub struct Account(InternedString);

impl Account {
    /// The parent account, if there is one.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.0
            .rfind(SEPARATOR)
            .map(|index| Self::from(&self.0[0..index]))
    }

    /// The account components.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split(SEPARATOR)
    }

    /// Get the root account.
    #[must_use]
    fn root(&self) -> &str {
        self.0
            .find(SEPARATOR)
            .map_or(&self.0, |index| &self.0[0..index])
    }

    /// Check whether the account name has a valid root.
    #[must_use]
    pub(crate) fn has_valid_root(&self, roots: &RootAccounts) -> bool {
        let root = self.root();
        root == roots.assets
            || root == roots.liabilities
            || root == roots.equity
            || root == roots.income
            || root == roots.expenses
    }

    /// Check whether the account name has valid syntax.
    ///
    /// A valid account name:
    /// - Has at least 2 components (root + subaccount)
    /// - Root component starts with uppercase letter, followed by letters, digits, or hyphens
    /// - Other components start with uppercase letter or digit, followed by letters, digits, or hyphens
    #[must_use]
    pub fn has_valid_name(&self) -> bool {
        ACCOUNT_RE.is_match(&self.0)
    }
}

/// Regex for valid account names.
static ACCOUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\p{Lu}][\p{L}\p{Nd}\-]*(:([\p{Lu}\p{Nd}][\p{L}\p{Nd}\-]*))+$")
        .expect("valid account regex")
});

impl Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str: &str = &self.0;
        f.debug_tuple("Account").field(&str).finish()
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<&str> for Account {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

pub(crate) trait JoinAccount {
    /// Join a subaccount name to an account.
    #[must_use]
    fn join_account(&self, child: &str) -> Account;
}

/// Keep roots as plain strings - they're not cloned a lot so there's no need for interning
type RootAccount = String;

impl JoinAccount for &RootAccount {
    fn join_account(&self, child: &str) -> Account {
        let mut self_str = (*self).clone();
        self_str.push(SEPARATOR);
        self_str.push_str(child);
        Account(self_str.into())
    }
}

/// The five root accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct RootAccounts {
    /// The root account for assets.
    pub assets: RootAccount,
    /// The root account for liabilities.
    pub liabilities: RootAccount,
    /// The root account for equity.
    pub equity: RootAccount,
    /// The root account for income.
    pub income: RootAccount,
    /// The root account for expenses.
    pub expenses: RootAccount,
}

impl Default for RootAccounts {
    fn default() -> Self {
        Self {
            assets: "Assets".into(),
            liabilities: "Liabilities".into(),
            equity: "Equity".into(),
            income: "Income".into(),
            expenses: "Expenses".into(),
        }
    }
}

impl RootAccounts {
    /// Whether the given account is an balance sheet account (either Assets, Liabilities or Equity).
    #[must_use]
    pub fn is_balance_sheet_account(&self, account: &Account) -> bool {
        let root = account.root();
        root == self.assets || root == self.liabilities || root == self.equity
    }
    /// Whether the given account is an income statement account (either Income or Expenses).
    #[must_use]
    pub fn is_income_statement_account(&self, account: &Account) -> bool {
        let root = account.root();
        root == self.income || root == self.expenses
    }
}

/// The accounts that are used in summarizations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarizationAccounts {
    /// The five root accounts
    pub roots: RootAccounts,
    /// Account to accumulate currency conversion for the reporting interval (subaccount of Equity).
    pub current_conversions: Account,
    /// Account to accumulate all previous earnings (subaccount of Equity).
    pub current_earnings: Account,
    /// Account to accumulate all previous account balances (subaccount of Equity).
    pub previous_balances: Account,
    /// Account to accumulate previous currency conversions (subaccount of Equity).
    pub previous_conversions: Account,
    /// Account that previous Income will be accumulated under (subaccount of Equity).
    pub previous_earnings: Account,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_filters() {
        let roots = RootAccounts::default();
        let acc: Account = "Income:Cash".into();
        assert!(!roots.is_balance_sheet_account(&acc));
        assert!(roots.is_income_statement_account(&acc));
        let acc: Account = "Equity:Opening".into();
        assert!(roots.is_balance_sheet_account(&acc));
        assert!(!roots.is_income_statement_account(&acc));
    }

    #[test]
    fn test_account_parent() {
        let root: Account = "Assets".into();
        assert_eq!(root.parent(), None);
        let acc: Account = "Assets:Cash".into();
        assert_eq!(acc.parent(), Some(root));
    }

    #[test]
    fn test_account_root() {
        let root: Account = "Assets".into();
        assert_eq!(root.root(), "Assets");
        let acc: Account = "Assets:Cash".into();
        assert_eq!(acc.root(), "Assets");
    }

    #[test]
    fn test_account_is_valid() {
        let roots = RootAccounts::default();
        let acc: Account = "Assets:Cash".into();
        assert!(acc.has_valid_root(&roots));
        let acc: Account = "Expenses:Cash".into();
        assert!(acc.has_valid_root(&roots));
        let acc: Account = "NotARoot:Cash".into();
        assert!(!acc.has_valid_root(&roots));
    }

    #[test]
    fn test_account_components() {
        let acc: Account = "Assets:US:Bank:Checking".into();
        let components: Vec<_> = acc.components().collect();
        assert_eq!(components, vec!["Assets", "US", "Bank", "Checking"]);
    }

    #[test]
    fn test_account_join() {
        let root = &"Assets".to_string();
        let acc: Account = "Assets:Cash".into();
        let acc_sub: Account = "Assets:Cash:Sub".into();
        assert_eq!(root.join_account("Cash"), acc);
        assert_eq!(root.join_account("Cash:Sub"), acc_sub);
    }

    #[test]
    fn test_has_valid_name() {
        // Valid account names
        assert!(Account::from("Assets:Cash").has_valid_name());
        assert!(Account::from("Assets:US:RBS:Checking").has_valid_name());
        assert!(Account::from("Equity:Opening-Balances").has_valid_name());
        assert!(Account::from("Income:US:ETrade:Dividends-USD").has_valid_name());
        assert!(Account::from("Assets:401k").has_valid_name()); // digit in subaccount start
        assert!(Account::from("Assets:2024-Savings").has_valid_name()); // digit start with hyphen

        // Invalid: only one component (no subaccount)
        assert!(!Account::from("Assets").has_valid_name());
        assert!(!Account::from("Income").has_valid_name());

        // Invalid: lowercase in component start
        assert!(!Account::from("Assets:cash").has_valid_name());
        assert!(!Account::from("Assets:US:rbs").has_valid_name());

        // Invalid: lowercase root
        assert!(!Account::from("assets:Cash").has_valid_name());

        // Invalid: special characters
        assert!(!Account::from("Assets:US*RBS").has_valid_name());
        assert!(!Account::from("Assets:US.RBS").has_valid_name());
        assert!(!Account::from("Assets:US_RBS").has_valid_name());

        // Valid: Unicode uppercase letters
        assert!(Account::from("Активы:Наличные").has_valid_name()); // Russian
        assert!(Account::from("Vermögen:Bank").has_valid_name()); // German umlaut in middle
        assert!(Account::from("Assets:Épargne").has_valid_name()); // French É

        // Invalid: Unicode lowercase start
        assert!(!Account::from("Assets:наличные").has_valid_name()); // Russian lowercase
        assert!(!Account::from("Assets:épargne").has_valid_name()); // French lowercase é
    }
}
