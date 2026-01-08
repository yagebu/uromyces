//! The data types that are used for handling Beancount data.
//!
//! We both have some base data types as well as types for the different kinds of Beancount entry.
//!
//! To be able to apply various optimisations and properly distinguish between them, basic
//! string-like types like [`Currency`] and [`Account`] each have their own wrapper type. With
//! their help, we can use string interners and easily make specific methods (like getting the
//! parent for an account) available. All of these data types can be serialised with `serde`.
//!
//! ## Entries
//!
//! Entries are the central composite data structure for transactions, documents and the other
//! types of Beancount input data. Each entry type exists as a separate struct:
//!
//! - [`Balance`]
//! - [`Close`]
//! - [`Commodity`]
//! - [`Custom`]
//! - [`Document`]
//! - [`Event`]
//! - [`Note`]
//! - [`Open`]
//! - [`Pad`]
//! - [`Price`]
//! - [`Query`]
//! - [`Transaction`]
//!
//! Before booking, various attributes on the postings of a transaction might still be missing,
//! such an unbooked transaction is represented by the [`RawTransaction`].
//!
//! To handle collections of entries, we have the enum [`Entry`], which has the above list of
//! different entry structs as its variants. To represented entries before booking, the
//! [`RawEntry`] enum represents mostly the same by contains a raw transaction instead of a fully
//! booked transaction.
//!
//! ## Base data types
//!
//! - [`Account`] - an account name
//! - [`Booking`] - an enumeration of the possible booking methods for accounts
//! - [`Currency`] - a currency name
//! - [`Date`] - a simple date
//! - [`Decimal`] - all numbers that represent some financial value.
//! - [`Filename`] - a file path - ensured to be absolute and valid unicode
//!   (or a dummy like `<summarize>`)
//! - [`Flag`] - an enumeration of the possible transaction or posting flags
//!
//! ## Base composite data types
//!
//! - [`Amount`] - an amount, a number of some currency

use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use pyo3::types::{PyBool, PyDate, PyDict, PyString};
use pyo3::{PyTypeInfo, prelude::*};
use serde::{Deserialize, Serialize};

mod account;
mod amount;
mod booking;
mod box_str;
mod convert_to_beancount;
mod cost;
mod currency;
mod date;
mod decimal;
mod flag;
mod interned_string;
mod metadata;
mod paths;
mod tags_links;

pub use account::{Account, RootAccounts, SummarizationAccounts};
pub use amount::{Amount, IncompleteAmount};
pub use booking::Booking;
pub use box_str::BoxStr;
pub use cost::{Cost, CostLabel, CostSpec};
pub use currency::Currency;
pub use date::{Date, MIN_DATE};
pub use decimal::Decimal;
pub use flag::Flag;
pub use metadata::{EntryHeader, Meta, MetaKeyValuePair, MetaValue};
pub use paths::{AbsoluteUTF8Path, Filename};
pub use tags_links::TagsLinks;

use decimal::get_decimal_decimal;

use crate::types::convert_to_beancount::ConvertToBeancount;

/// The type to use for line numbers in file positions.
pub type LineNumber = u32;
pub type Payee = Option<BoxStr>;
pub type Narration = BoxStr;

/// A raw Beancount directive (option, plugin, or include).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawDirective {
    /// A raw Beancount option.
    Option {
        filename: Filename,
        line: LineNumber,
        key: String,
        value: String,
    },
    Plugin {
        name: String,
        config: Option<String>,
    },
    Include {
        pattern: String,
    },
}

/// A plugin directive.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Plugin {
    /// The plugin name - name of a Python module that contains plugin functions in `__plugins__`.
    #[pyo3(get)]
    pub name: String,
    /// Optionally, config for the plugin.
    #[pyo3(get)]
    pub config: Option<String>,
}

/// A custom value - a value and associated type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
#[serde(transparent)]
pub struct CustomValue(pub(crate) MetaValue);

#[pymethods]
impl CustomValue {
    #[new]
    fn __new__(py: Python<'_>, value: MetaValue, dtype: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let MetaValue::String(s) = &value {
            let account_dtype = pyo3::intern!(py, "<AccountDummy>");
            if dtype.eq(account_dtype)? {
                return Ok(Self(MetaValue::Account(s.as_str().into())));
            }
        }
        Ok(Self(value))
    }

    #[getter]
    fn value(&self) -> &MetaValue {
        &self.0
    }

    #[getter]
    fn dtype<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(match self.0 {
            MetaValue::Currency(_) | MetaValue::String(_) | MetaValue::Tag(_) => {
                PyString::type_object(py).into_any()
            }
            MetaValue::Date(_) => PyDate::type_object(py).into_any(),
            MetaValue::Account(_) => pyo3::intern!(py, "<AccountDummy>").clone().into_any(),
            MetaValue::Bool(_) => PyBool::type_object(py).into_any(),
            MetaValue::Amount(_) => Amount::type_object(py).into_any(),
            MetaValue::Number(_) => get_decimal_decimal(py)?.clone().into_any(),
        })
    }
}

/// A balance entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Balance {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub amount: Amount,
    #[pyo3(get)]
    pub tolerance: Option<Decimal>,
}

/// An account close entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Close {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
}

/// A commodity entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Commodity {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub currency: Currency,
}

/// A custom entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Custom {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub r#type: String,
    #[pyo3(get)]
    pub values: Vec<CustomValue>,
}

/// An document entry for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Document {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub filename: AbsoluteUTF8Path,
}

/// An event for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Event {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub r#type: String,
    #[pyo3(get)]
    pub description: String,
}

/// An account open entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Open {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub currencies: Vec<Currency>,
    #[pyo3(get)]
    pub booking: Option<Booking>,
}

/// A note entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Note {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub comment: String,
}

/// A pad entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Pad {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub source_account: Account,
}

/// A price entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Price {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub currency: Currency,
    #[pyo3(get)]
    pub amount: Amount,
}

/// A raw posting (pre-booking), which might be missing some attributes.
///
/// During booking, the incomplete amounts will be replaced with the actual amounts
/// and the cost spec will turn into a cost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPosting {
    /// The filename.
    pub filename: Filename,
    /// The 1-based line number.
    pub line: LineNumber,
    pub meta: Meta,

    pub account: Account,
    pub flag: Option<Flag>,
    pub units: IncompleteAmount,
    pub price: Option<IncompleteAmount>,
    pub cost: Option<CostSpec>,
}

impl RawPosting {
    /// Complete the posting with the given units, cost, and price.
    pub(crate) fn complete(
        self,
        units: Amount,
        price: Option<Amount>,
        cost: Option<Cost>,
    ) -> Posting {
        Posting {
            filename: Some(self.filename),
            line: Some(self.line),
            meta: self.meta,
            account: self.account,
            flag: self.flag,
            units,
            cost,
            price,
        }
    }
}

/// A fully booked posting.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Posting {
    /// The filename.
    pub filename: Option<Filename>,
    /// The 1-based line number.
    pub line: Option<LineNumber>,
    /// Metadata for the posting.
    pub meta: Meta,

    /// The account that the posting should be booked to.
    #[pyo3(get)]
    pub account: Account,
    /// The units of the posting.
    #[pyo3(get)]
    pub units: Amount,
    /// An optional price for the units of the posting.
    #[pyo3(get)]
    pub price: Option<Amount>,
    /// An optional cost for the units of the posting.
    #[pyo3(get)]
    pub cost: Option<Cost>,
    /// An optional flag for the posting.
    #[pyo3(get)]
    pub flag: Option<Flag>,
}

#[pymethods]
impl Posting {
    #[new]
    #[pyo3(signature = (account, units, cost=None, price=None, flag=None, meta=None))]
    fn __new__(
        account: Account,
        units: Amount,
        cost: Option<Cost>,
        price: Option<Amount>,
        flag: Option<Flag>,
        meta: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let (filename, line, meta) = match meta {
            Some(meta) => metadata::extract_meta_dict(meta)?,
            None => (None, None, Meta::default()),
        };
        Ok(Self {
            filename,
            line,
            meta,
            account,
            units,
            price,
            cost,
            flag,
        })
    }
    #[getter]
    fn meta<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.meta.to_py_dict(py, self.filename.as_ref(), self.line)
    }
}

impl Posting {
    /// Create a posting for an account with just some units.
    #[must_use]
    pub(crate) fn new_simple(filename: Filename, account: Account, units: Amount) -> Self {
        Self {
            flag: None,
            filename: Some(filename),
            line: None,
            account,
            units,
            cost: None,
            price: None,
            meta: Meta::default(),
        }
    }

    /// Create a posting for an account with just units and possibly a cost.
    #[must_use]
    pub(crate) fn new_with_cost(
        filename: Filename,
        account: Account,
        units: Amount,
        cost: Option<Cost>,
    ) -> Self {
        Self {
            flag: None,
            filename: Some(filename),
            line: None,
            account,
            units,
            cost,
            price: None,
            meta: Meta::default(),
        }
    }
}

/// A transaction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Transaction {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub flag: Flag,
    #[pyo3(get)]
    pub payee: Payee,
    #[pyo3(get)]
    pub narration: Narration,
    #[pyo3(get)]
    pub postings: Vec<Posting>,
}

impl Transaction {
    /// Create a transaction.
    ///
    /// Generic to allow &str, String, etc. to be used for narration.
    #[must_use]
    pub fn new<T: Into<Narration>>(
        header: EntryHeader,
        flag: Flag,
        payee: Payee,
        narration: T,
        postings: Vec<Posting>,
    ) -> Self {
        Self {
            header,
            flag,
            payee,
            narration: narration.into(),
            postings,
        }
    }
}

/// A raw transaction.
///
/// After parsing, parts of the transaction might still be missing and will
/// only be inferred during booking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTransaction {
    pub header: EntryHeader,
    pub flag: Flag,
    pub payee: Payee,
    pub narration: Narration,
    pub postings: Vec<RawPosting>,
}

impl RawTransaction {
    /// Complete the transaction with the given booked postings.
    pub(crate) fn complete(self, postings: Vec<Posting>) -> Transaction {
        Transaction::new(self.header, self.flag, self.payee, self.narration, postings)
    }
}

/// A query entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Query {
    #[serde(flatten)]
    #[pyo3(get, name = "meta")]
    pub header: EntryHeader,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub query_string: String,
}

/// The Beancount entries (raw, after parsing).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum RawEntry {
    Balance(Balance),
    Close(Close),
    Commodity(Commodity),
    Custom(Custom),
    Document(Document),
    Event(Event),
    Note(Note),
    Open(Open),
    Pad(Pad),
    Price(Price),
    Query(Query),
    Transaction(RawTransaction),
}

/// The Beancount entries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "t")]
#[derive(FromPyObject, IntoPyObject)]
pub enum Entry {
    Balance(Balance),
    Close(Close),
    Commodity(Commodity),
    Custom(Custom),
    Document(Document),
    Event(Event),
    Note(Note),
    Open(Open),
    Pad(Pad),
    Price(Price),
    Query(Query),
    Transaction(Transaction),
}

#[pymethods]
impl Balance {
    #[new]
    #[pyo3(signature = (header, account, amount, tolerance=None))]
    fn __new__(
        header: EntryHeader,
        account: Account,
        amount: Amount,
        tolerance: Option<Decimal>,
    ) -> Self {
        Self {
            header,
            account,
            amount,
            tolerance,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None, amount=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        amount: Option<Amount>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
            amount: amount.unwrap_or_else(|| self.amount.clone()),
            tolerance: self.tolerance,
        })
    }
}
#[pymethods]
impl Close {
    #[new]
    fn __new__(header: EntryHeader, account: Account) -> Self {
        Self { header, account }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
        })
    }
}
#[pymethods]
impl Commodity {
    #[new]
    fn __new__(header: EntryHeader, currency: Currency) -> Self {
        Self { header, currency }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, currency=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        currency: Option<Currency>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            currency: currency.unwrap_or_else(|| self.currency.clone()),
        })
    }
}
#[pymethods]
impl Custom {
    #[new]
    fn __new__(header: EntryHeader, r#type: String, values: Vec<CustomValue>) -> Self {
        Self {
            header,
            r#type,
            values,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, r#type=None, values=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        r#type: Option<String>,
        values: Option<Vec<CustomValue>>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            r#type: r#type.unwrap_or_else(|| self.r#type.clone()),
            values: values.unwrap_or_else(|| self.values.clone()),
        })
    }
}
#[pymethods]
impl Document {
    #[new]
    fn __new__(header: EntryHeader, account: Account, filename: AbsoluteUTF8Path) -> Self {
        Self {
            header,
            account,
            filename,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None, filename=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        filename: Option<AbsoluteUTF8Path>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
            filename: filename.unwrap_or_else(|| self.filename.clone()),
        })
    }
}
#[pymethods]
impl Event {
    #[new]
    fn __new__(header: EntryHeader, r#type: String, description: String) -> Self {
        Self {
            header,
            r#type,
            description,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, r#type=None, description=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        r#type: Option<String>,
        description: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            r#type: r#type.unwrap_or_else(|| self.r#type.clone()),
            description: description.unwrap_or_else(|| self.description.clone()),
        })
    }
}
#[pymethods]
impl Note {
    #[new]
    fn __new__(header: EntryHeader, account: Account, comment: String) -> Self {
        Self {
            header,
            account,
            comment,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None, comment=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        comment: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
            comment: comment.unwrap_or_else(|| self.comment.clone()),
        })
    }
}
#[pymethods]
impl Open {
    #[new]
    #[pyo3(signature = (header, account, currencies, booking=None))]
    fn __new__(
        header: EntryHeader,
        account: Account,
        currencies: Vec<Currency>,
        booking: Option<Booking>,
    ) -> Self {
        Self {
            header,
            account,
            currencies,
            booking,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None, currencies=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        currencies: Option<Vec<Currency>>,
        // TODO: booking: Option<Booking>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
            currencies: currencies.unwrap_or_else(|| self.currencies.clone()),
            booking: self.booking,
        })
    }
}
#[pymethods]
impl Pad {
    #[new]
    fn __new__(header: EntryHeader, account: Account, source_account: Account) -> Self {
        Self {
            header,
            account,
            source_account,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, account=None, source_account=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        source_account: Option<Account>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            account: account.unwrap_or_else(|| self.account.clone()),
            source_account: source_account.unwrap_or_else(|| self.source_account.clone()),
        })
    }
}
#[pymethods]
impl Price {
    #[new]
    fn __new__(header: EntryHeader, currency: Currency, amount: Amount) -> Self {
        Self {
            header,
            currency,
            amount,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, currency=None, amount=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        currency: Option<Currency>,
        amount: Option<Amount>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            currency: currency.unwrap_or_else(|| self.currency.clone()),
            amount: amount.unwrap_or_else(|| self.amount.clone()),
        })
    }
}
#[pymethods]
impl Query {
    #[new]
    fn __new__(header: EntryHeader, name: String, query_string: String) -> Self {
        Self {
            header,
            name,
            query_string,
        }
    }

    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, name=None, query_string=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        name: Option<String>,
        query_string: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            name: name.unwrap_or_else(|| self.name.clone()),
            query_string: query_string.unwrap_or_else(|| self.query_string.clone()),
        })
    }
}
#[pymethods]
impl Transaction {
    #[new]
    fn __new__(
        header: EntryHeader,
        flag: Flag,
        payee: Payee,
        narration: Narration,
        postings: Vec<Posting>,
    ) -> Self {
        Self {
            header,
            flag,
            payee,
            narration,
            postings,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (*, date=None, meta=None, tags=None, links=None, flag=None, payee=None, narration=None, postings=None))]
    fn _replace(
        &self,
        date: Option<Date>,
        meta: Option<&Bound<'_, PyDict>>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        flag: Option<Flag>,
        payee: Option<BoxStr>,
        narration: Option<Narration>,
        postings: Option<Vec<Posting>>,
    ) -> PyResult<Self> {
        Ok(Self {
            header: self
                .header
                .replace_meta_tags_links(date, meta, tags, links)?,
            flag: flag.unwrap_or(self.flag),
            payee: payee.or_else(|| self.payee.clone()),
            narration: narration.unwrap_or_else(|| self.narration.clone()),
            postings: postings.unwrap_or_else(|| self.postings.clone()),
        })
    }
}

/// Since all the entry types need the same additional getter functions, this macro provides them.
macro_rules! pymethods_for_entry {
    ($a:ident) => {
        #[pymethods]
        impl $a {
            #[getter]
            fn date(&self) -> &Date {
                &self.header.date
            }
            #[getter]
            fn links(&self) -> &TagsLinks {
                &self.header.links
            }
            #[getter]
            fn tags(&self) -> &TagsLinks {
                &self.header.tags
            }
            fn __repr__(&self) -> String {
                format!("<{:?}>", self)
            }
            fn __hash__(&self) -> u64 {
                // use a fixed hash function here and not the Rust DefaultHasher to keep it stable
                let mut hasher = ahash::AHasher::default();
                self.hash(&mut hasher);
                hasher.finish()
            }
            fn to_json(&self) -> String {
                serde_json::to_string(&Into::<Entry>::into(self.clone())).unwrap()
            }
            fn _convert<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
                self.convert_to_beancount(py)
            }
        }
    };
}

pymethods_for_entry!(Balance);
pymethods_for_entry!(Close);
pymethods_for_entry!(Commodity);
pymethods_for_entry!(Custom);
pymethods_for_entry!(Document);
pymethods_for_entry!(Event);
pymethods_for_entry!(Note);
pymethods_for_entry!(Open);
pymethods_for_entry!(Pad);
pymethods_for_entry!(Price);
pymethods_for_entry!(Query);
pymethods_for_entry!(Transaction);

impl Entry {
    /// Get the entry header.
    #[must_use]
    pub fn get_header(&self) -> &EntryHeader {
        match self {
            Self::Balance(e) => &e.header,
            Self::Close(e) => &e.header,
            Self::Commodity(e) => &e.header,
            Self::Custom(e) => &e.header,
            Self::Document(e) => &e.header,
            Self::Event(e) => &e.header,
            Self::Note(e) => &e.header,
            Self::Open(e) => &e.header,
            Self::Pad(e) => &e.header,
            Self::Price(e) => &e.header,
            Self::Query(e) => &e.header,
            Self::Transaction(e) => &e.header,
        }
    }

    /// Sort key for an entry.
    ///
    /// Is used to implement the [`Ord`] and [`PartialOrd`] traits below.
    ///
    /// Entries are sorted by date, and on a day are sorted as follows:
    ///
    /// - Open
    /// - Balance
    /// - ... all others
    /// - Document
    /// - Close
    fn sort_key(&self) -> (&Date, i8) {
        match self {
            Self::Balance(e) => (&e.header.date, -1),
            Self::Close(e) => (&e.header.date, 2),
            Self::Commodity(e) => (&e.header.date, 0),
            Self::Custom(e) => (&e.header.date, 0),
            Self::Document(e) => (&e.header.date, 1),
            Self::Event(e) => (&e.header.date, 0),
            Self::Note(e) => (&e.header.date, 0),
            Self::Open(e) => (&e.header.date, -2),
            Self::Pad(e) => (&e.header.date, 0),
            Self::Price(e) => (&e.header.date, 0),
            Self::Query(e) => (&e.header.date, 0),
            Self::Transaction(e) => (&e.header.date, 0),
        }
    }

    /// Get the accounts for the entry.
    #[must_use]
    pub fn get_accounts(&self) -> Vec<&Account> {
        match self {
            Self::Balance(e) => vec![&e.account],
            Self::Close(e) => vec![&e.account],
            Self::Commodity(..)
            | Self::Custom(..)
            | Self::Event(..)
            | Self::Price(..)
            | Self::Query(..) => Vec::new(),
            Self::Document(e) => vec![&e.account],
            Self::Note(e) => vec![&e.account],
            Self::Open(e) => vec![&e.account],
            Self::Pad(e) => vec![&e.account, &e.source_account],
            Self::Transaction(e) => e.postings.iter().map(|p| &p.account).collect(),
        }
    }
}

impl RawEntry {
    /// Get the entry header.
    #[must_use]
    pub fn get_header(&self) -> &EntryHeader {
        match self {
            Self::Balance(e) => &e.header,
            Self::Close(e) => &e.header,
            Self::Commodity(e) => &e.header,
            Self::Custom(e) => &e.header,
            Self::Document(e) => &e.header,
            Self::Event(e) => &e.header,
            Self::Note(e) => &e.header,
            Self::Open(e) => &e.header,
            Self::Pad(e) => &e.header,
            Self::Price(e) => &e.header,
            Self::Query(e) => &e.header,
            Self::Transaction(e) => &e.header,
        }
    }

    /// Sort key for an entry.
    ///
    /// Is used to implement the `[Ord]` and `[PartialOrd]` traits below.
    ///
    /// Entries are sorted by date, and on a day are sorted as follows:
    ///
    /// - Open
    /// - Balance
    /// - ... all others
    /// - Document
    /// - Close
    fn sort_key(&self) -> (&Date, i8) {
        match self {
            Self::Balance(e) => (&e.header.date, -1),
            Self::Close(e) => (&e.header.date, 2),
            Self::Commodity(e) => (&e.header.date, 0),
            Self::Custom(e) => (&e.header.date, 0),
            Self::Document(e) => (&e.header.date, 1),
            Self::Event(e) => (&e.header.date, 0),
            Self::Note(e) => (&e.header.date, 0),
            Self::Open(e) => (&e.header.date, -2),
            Self::Pad(e) => (&e.header.date, 0),
            Self::Price(e) => (&e.header.date, 0),
            Self::Transaction(e) => (&e.header.date, 0),
            Self::Query(e) => (&e.header.date, 0),
        }
    }
}

impl PartialOrd for RawEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RawEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// Macro to define the From trait for the enum for all the entry types.
macro_rules! raw_entry_enum_from_entry {
    // Implement the From trait to both RawEntry as well as Entry.
    ($entry_type:ident) => {
        impl From<$entry_type> for RawEntry {
            fn from(e: $entry_type) -> Self {
                RawEntry::$entry_type(e)
            }
        }
        impl From<$entry_type> for Entry {
            fn from(e: $entry_type) -> Self {
                Entry::$entry_type(e)
            }
        }
    };
    // For RawTransaction, the type and the name (Transaction) of the enum variant do not match.
    ($from_entry_type:ident, $to_entry_type:ident) => {
        impl From<$from_entry_type> for RawEntry {
            fn from(e: $from_entry_type) -> Self {
                RawEntry::$to_entry_type(e)
            }
        }
        impl From<$to_entry_type> for Entry {
            fn from(e: $to_entry_type) -> Self {
                Entry::$to_entry_type(e)
            }
        }
    };
}

raw_entry_enum_from_entry!(Balance);
raw_entry_enum_from_entry!(Close);
raw_entry_enum_from_entry!(Commodity);
raw_entry_enum_from_entry!(Custom);
raw_entry_enum_from_entry!(Document);
raw_entry_enum_from_entry!(Event);
raw_entry_enum_from_entry!(Note);
raw_entry_enum_from_entry!(Open);
raw_entry_enum_from_entry!(Pad);
raw_entry_enum_from_entry!(Price);
raw_entry_enum_from_entry!(Query);
raw_entry_enum_from_entry!(RawTransaction, Transaction);
