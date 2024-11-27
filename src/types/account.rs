use std::convert::Infallible;
use std::fmt::{Debug, Display};

use internment::ArcIntern;
use pyo3::{prelude::*, pybacked::PyBackedStr, types::PyString};
use serde::{Deserialize, Serialize};

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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Account(ArcIntern<String>);

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
    pub fn root(&self) -> Self {
        self.0
            .find(SEPARATOR)
            .map_or(self.clone(), |index| Self::from(&self.0[0..index]))
    }

    /// Check whether the account name has a valid root.
    #[must_use]
    pub fn has_valid_root(&self, roots: &RootAccounts) -> bool {
        let root = self.root();
        root == roots.assets
            || root == roots.liabilities
            || root == roots.equity
            || root == roots.income
            || root == roots.expenses
    }

    /// Join an account name.
    #[must_use]
    pub fn join(&self, child: &str) -> Self {
        let mut self_str = self.0.to_string();
        self_str.push(':');
        self_str.push_str(child);
        self_str.as_str().into()
    }
}

impl Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Account").field(&self.0.as_ref()).finish()
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.as_ref(), f)
    }
}

impl From<&str> for Account {
    fn from(s: &str) -> Self {
        Self(ArcIntern::from_ref(s))
    }
}

impl<'a, 'py> IntoPyObject<'py> for &'a Account {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0.as_str().into_pyobject(py)
    }
}

impl<'source> FromPyObject<'source> for Account {
    fn extract_bound(ob: &Bound<'source, PyAny>) -> PyResult<Self> {
        let str = ob.extract::<PyBackedStr>()?;
        Ok((&*str).into())
    }
}

/// The five root accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct RootAccounts {
    /// The root account for assets.
    #[pyo3(get)]
    pub assets: Account,
    /// The root account for liabilities.
    #[pyo3(get)]
    pub liabilities: Account,
    /// The root account for equity.
    #[pyo3(get)]
    pub equity: Account,
    /// The root account for income.
    #[pyo3(get)]
    pub income: Account,
    /// The root account for expenses.
    #[pyo3(get)]
    pub expenses: Account,
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
        assert_eq!(root.root(), "Assets".into());
        let acc: Account = "Assets:Cash".into();
        assert_eq!(acc.root(), "Assets".into());
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
    fn test_account_join() {
        let root: Account = "Assets".into();
        let acc: Account = "Assets:Cash".into();
        assert_eq!(root.join("Cash"), acc);
    }
}
