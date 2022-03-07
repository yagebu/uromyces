use std::fmt::{Debug, Display};
use std::ops::Deref;

use internment::ArcIntern;
use pyo3::{IntoPy, PyObject, ToPyObject};
use serde::{Deserialize, Serialize};

/// Components of the account are separated by colons.
const SEPARATOR: char = ':';

/// An account name.
///
/// An account name is a string where components of the account are separated by `:`. The account
/// name needs to start with one of the five root accounts. There are some further restrictions on
/// the syntax that is ensured by the parser.
///
/// To speed up common operations on account names, this uses a string interner.
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

impl Deref for Account {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for Account {
    fn from(s: &str) -> Self {
        Account(ArcIntern::from_ref(s))
    }
}

impl ToPyObject for Account {
    fn to_object(&self, py: pyo3::Python<'_>) -> PyObject {
        self.0.to_object(py)
    }
}

impl IntoPy<PyObject> for Account {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        self.0.to_object(py)
    }
}

/// The five root accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootAccounts {
    /// The root account for assets.
    pub assets: Account,
    /// The root account for liabilities.
    pub liabilities: Account,
    /// The root account for equity.
    pub equity: Account,
    /// The root account for income.
    pub income: Account,
    /// The root account for expenses.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
