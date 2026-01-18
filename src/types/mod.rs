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

use pyo3::types::{PyBool, PyDate, PyInt, PyString};
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

pub(crate) use account::JoinAccount;
pub use account::{Account, RootAccounts, SummarizationAccounts};
pub use amount::{Amount, IncompleteAmount};
pub use booking::Booking;
pub use box_str::BoxStr;
pub use cost::{Cost, CostLabel, CostSpec};
pub use currency::Currency;
pub use date::{Date, MIN_DATE};
pub use decimal::Decimal;
pub use flag::Flag;
pub use metadata::{EntryMeta, Meta, MetaKeyValuePair, MetaValue, PostingMeta};
pub use paths::{AbsoluteUTF8Path, Filename};
pub use tags_links::TagsLinks;

use convert_to_beancount::ConvertToBeancount;
use decimal::get_decimal_decimal;

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
        lineno: LineNumber,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Plugin {
    /// The plugin name - name of a Python module that contains plugin functions in `__plugins__`.
    pub name: String,
    /// Optionally, config for the plugin.
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
            MetaValue::Decimal(_) => get_decimal_decimal(py)?.clone().into_any(),
            MetaValue::Integer(_) => PyInt::type_object(py).into_any(),
        })
    }
}

/// A raw posting (pre-booking), which might be missing some attributes.
///
/// During booking, the incomplete amounts will be replaced with the actual amounts
/// and the cost spec will turn into a cost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPosting {
    /// Metadata for the posting.
    pub meta: EntryMeta,

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
            meta: self.meta.into(),
            account: self.account,
            flag: self.flag,
            units,
            cost,
            price,
        }
    }
}

/// A raw transaction.
///
/// After parsing, parts of the transaction might still be missing and will
/// only be inferred during booking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTransaction {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub flag: Flag,
    pub payee: Payee,
    pub narration: Narration,
    pub postings: Vec<RawPosting>,
}

impl RawTransaction {
    /// Complete the transaction with the given booked postings.
    pub(crate) fn complete(self, postings: Vec<Posting>) -> Transaction {
        Transaction::new(
            self.meta,
            self.date,
            self.tags,
            self.links,
            self.flag,
            self.payee,
            self.narration,
            postings,
        )
    }
}

/// A fully booked posting.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, module = "uromyces")]
pub struct Posting {
    /// Metadata for the posting.
    #[pyo3(get)]
    pub meta: PostingMeta,

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
        meta: Option<PostingMeta>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_default(),
            account,
            units,
            price,
            cost,
            flag,
        }
    }
}

impl Posting {
    /// Create a posting for an account with just some units.
    #[must_use]
    pub(crate) fn new_simple(filename: Filename, account: Account, units: Amount) -> Self {
        Self {
            flag: None,
            meta: PostingMeta::with_filename(filename),
            account,
            units,
            cost: None,
            price: None,
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
            meta: PostingMeta::with_filename(filename),
            account,
            units,
            cost,
            price: None,
        }
    }
}

// -----------------------------------------------------------------
/// A balance entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Balance {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
    pub amount: Amount,
    pub tolerance: Option<Decimal>,
}

/// An account close entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Close {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
}

/// A commodity entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Commodity {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub currency: Currency,
}

/// A custom entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Custom {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub r#type: String,
    pub values: Vec<CustomValue>,
}

/// An document entry for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Document {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
    pub filename: AbsoluteUTF8Path,
}

/// An event for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Event {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub r#type: String,
    pub description: String,
}

/// An account open entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Open {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
    pub currencies: Vec<Currency>,
    pub booking: Option<Booking>,
}

/// A note entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Note {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
    pub comment: String,
}

/// A pad entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Pad {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub account: Account,
    pub source_account: Account,
}

/// A price entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Price {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub currency: Currency,
    pub amount: Amount,
}

/// A query entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Query {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub name: String,
    pub query_string: String,
}

/// A transaction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, eq, get_all, module = "uromyces")]
pub struct Transaction {
    pub meta: EntryMeta,
    pub date: Date,
    pub tags: TagsLinks,
    pub links: TagsLinks,
    pub flag: Flag,
    pub payee: Payee,
    pub narration: Narration,
    pub postings: Vec<Posting>,
}

impl Transaction {
    /// Create a transaction.
    ///
    /// Generic to allow &str, String, etc. to be used for narration.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new<T: Into<Narration>>(
        meta: EntryMeta,
        date: Date,
        tags: TagsLinks,
        links: TagsLinks,
        flag: Flag,
        payee: Payee,
        narration: T,
        postings: Vec<Posting>,
    ) -> Self {
        Self {
            date,
            tags,
            links,
            meta,
            flag,
            payee,
            narration: narration.into(),
            postings,
        }
    }
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

    RawTransaction(RawTransaction),
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
    #[pyo3(signature = (meta, date, account, amount, tolerance=None, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        amount: Amount,
        tolerance: Option<Decimal>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
            amount,
            tolerance,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None, amount=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        amount: Option<Amount>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
            amount: amount.unwrap_or_else(|| self.amount.clone()),
            tolerance: self.tolerance,
        }
    }
}
#[pymethods]
impl Close {
    #[new]
    #[pyo3(signature = (meta, date, account, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
        }
    }
}
#[pymethods]
impl Commodity {
    #[new]
    #[pyo3(signature = (meta, date, currency, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        currency: Currency,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            currency,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, currency=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        currency: Option<Currency>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            currency: currency.unwrap_or_else(|| self.currency.clone()),
        }
    }
}
#[pymethods]
impl Custom {
    #[new]
    #[pyo3(signature = (meta, date, r#type, values, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        r#type: String,
        values: Vec<CustomValue>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            r#type,
            values,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, r#type=None, values=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        r#type: Option<String>,
        values: Option<Vec<CustomValue>>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            r#type: r#type.unwrap_or_else(|| self.r#type.clone()),
            values: values.unwrap_or_else(|| self.values.clone()),
        }
    }
}
#[pymethods]
impl Document {
    #[new]
    #[pyo3(signature = (meta, date, account, filename, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        filename: AbsoluteUTF8Path,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
            filename,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None, filename=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        filename: Option<AbsoluteUTF8Path>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
            filename: filename.unwrap_or_else(|| self.filename.clone()),
        }
    }
}
#[pymethods]
impl Event {
    #[new]
    #[pyo3(signature = (meta, date, r#type, description, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        r#type: String,
        description: String,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            r#type,
            description,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, r#type=None, description=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        r#type: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            r#type: r#type.unwrap_or_else(|| self.r#type.clone()),
            description: description.unwrap_or_else(|| self.description.clone()),
        }
    }
}
#[pymethods]
impl Note {
    #[new]
    #[pyo3(signature = (meta, date, account, comment, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        comment: String,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
            comment,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None, comment=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        comment: Option<String>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
            comment: comment.unwrap_or_else(|| self.comment.clone()),
        }
    }
}
#[pymethods]
impl Open {
    #[new]
    #[pyo3(signature = (meta, date, account, currencies, booking=None, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        currencies: Vec<Currency>,
        booking: Option<Booking>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
            currencies,
            booking,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None, currencies=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        currencies: Option<Vec<Currency>>,
        // TODO: booking: Option<Booking>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
            currencies: currencies.unwrap_or_else(|| self.currencies.clone()),
            booking: self.booking,
        }
    }
}
#[pymethods]
impl Pad {
    #[new]
    #[pyo3(signature = (meta, date, account, source_account, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        account: Account,
        source_account: Account,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            account,
            source_account,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, account=None, source_account=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        account: Option<Account>,
        source_account: Option<Account>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            account: account.unwrap_or_else(|| self.account.clone()),
            source_account: source_account.unwrap_or_else(|| self.source_account.clone()),
        }
    }
}
#[pymethods]
impl Price {
    #[new]
    #[pyo3(signature = (meta, date, currency, amount, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        currency: Currency,
        amount: Amount,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            currency,
            amount,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, currency=None, amount=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        currency: Option<Currency>,
        amount: Option<Amount>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            currency: currency.unwrap_or_else(|| self.currency.clone()),
            amount: amount.unwrap_or_else(|| self.amount.clone()),
        }
    }
}
#[pymethods]
impl Query {
    #[new]
    #[pyo3(signature = (meta, date, name, query_string, tags=None, links=None))]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        name: String,
        query_string: String,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            name,
            query_string,
        }
    }

    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, name=None, query_string=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        name: Option<String>,
        query_string: Option<String>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            name: name.unwrap_or_else(|| self.name.clone()),
            query_string: query_string.unwrap_or_else(|| self.query_string.clone()),
        }
    }
}
#[pymethods]
impl Transaction {
    #[new]
    #[pyo3(signature = (meta, date, flag, payee, narration, postings, tags=None, links=None))]
    #[allow(clippy::too_many_arguments)]
    fn __new__(
        meta: EntryMeta,
        date: Date,
        flag: Flag,
        payee: Payee,
        narration: Narration,
        postings: Vec<Posting>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
    ) -> Self {
        Self {
            date,
            tags: tags.unwrap_or_default(),
            links: links.unwrap_or_default(),
            meta,
            flag,
            payee,
            narration,
            postings,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (*, meta=None, date=None, tags=None, links=None, flag=None, payee=None, narration=None, postings=None))]
    fn _replace(
        &self,
        meta: Option<EntryMeta>,
        date: Option<Date>,
        tags: Option<TagsLinks>,
        links: Option<TagsLinks>,
        flag: Option<Flag>,
        payee: Option<BoxStr>,
        narration: Option<Narration>,
        postings: Option<Vec<Posting>>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_else(|| self.meta.clone()),
            date: date.unwrap_or(self.date),
            tags: tags.unwrap_or_else(|| self.tags.clone()),
            links: links.unwrap_or_else(|| self.links.clone()),
            flag: flag.unwrap_or(self.flag),
            payee: payee.or_else(|| self.payee.clone()),
            narration: narration.unwrap_or_else(|| self.narration.clone()),
            postings: postings.unwrap_or_else(|| self.postings.clone()),
        }
    }
}

/// Since all the entry types need the same additional functions, this macro provides them.
macro_rules! pymethods_for_entry {
    ($a:ident) => {
        #[pymethods]
        impl $a {
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
    /// Get the entry metadata.
    #[must_use]
    pub(crate) fn meta(&self) -> &EntryMeta {
        match self {
            Self::Balance(e) => &e.meta,
            Self::Close(e) => &e.meta,
            Self::Commodity(e) => &e.meta,
            Self::Custom(e) => &e.meta,
            Self::Document(e) => &e.meta,
            Self::Event(e) => &e.meta,
            Self::Note(e) => &e.meta,
            Self::Open(e) => &e.meta,
            Self::Pad(e) => &e.meta,
            Self::Price(e) => &e.meta,
            Self::Query(e) => &e.meta,
            Self::Transaction(e) => &e.meta,
        }
    }

    /// Get the entry date.
    #[must_use]
    pub(crate) fn date(&self) -> Date {
        match self {
            Self::Balance(e) => e.date,
            Self::Close(e) => e.date,
            Self::Commodity(e) => e.date,
            Self::Custom(e) => e.date,
            Self::Document(e) => e.date,
            Self::Event(e) => e.date,
            Self::Note(e) => e.date,
            Self::Open(e) => e.date,
            Self::Pad(e) => e.date,
            Self::Price(e) => e.date,
            Self::Query(e) => e.date,
            Self::Transaction(e) => e.date,
        }
    }

    crate::macros::as_inner_method!(as_balance, Balance);
    crate::macros::as_inner_method!(as_document, Document);
    crate::macros::as_inner_method!(as_pad, Pad);
    #[cfg(test)]
    crate::macros::as_inner_method!(as_price, Price);
    crate::macros::as_inner_method!(as_transaction, Transaction);

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
            Self::Balance(e) => (&e.date, -1),
            Self::Close(e) => (&e.date, 2),
            Self::Commodity(e) => (&e.date, 0),
            Self::Custom(e) => (&e.date, 0),
            Self::Document(e) => (&e.date, 1),
            Self::Event(e) => (&e.date, 0),
            Self::Note(e) => (&e.date, 0),
            Self::Open(e) => (&e.date, -2),
            Self::Pad(e) => (&e.date, 0),
            Self::Price(e) => (&e.date, 0),
            Self::Query(e) => (&e.date, 0),
            Self::Transaction(e) => (&e.date, 0),
        }
    }

    /// Get the accounts for the entry.
    #[must_use]
    pub fn accounts(&self) -> Vec<&Account> {
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
    #[cfg(test)]
    crate::macros::as_inner_method!(as_transaction, RawTransaction);

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
            Self::Balance(e) => (&e.date, -1),
            Self::Close(e) => (&e.date, 2),
            Self::Commodity(e) => (&e.date, 0),
            Self::Custom(e) => (&e.date, 0),
            Self::Document(e) => (&e.date, 1),
            Self::Event(e) => (&e.date, 0),
            Self::Note(e) => (&e.date, 0),
            Self::Open(e) => (&e.date, -2),
            Self::Pad(e) => (&e.date, 0),
            Self::Price(e) => (&e.date, 0),
            Self::RawTransaction(e) => (&e.date, 0),
            Self::Query(e) => (&e.date, 0),
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

crate::macros::enum_from_inner!(
    RawEntry,
    Balance,
    Close,
    Commodity,
    Custom,
    Document,
    Event,
    Note,
    Open,
    Pad,
    Price,
    Query,
    RawTransaction
);
crate::macros::enum_from_inner!(
    Entry,
    Balance,
    Close,
    Commodity,
    Custom,
    Document,
    Event,
    Note,
    Open,
    Pad,
    Price,
    Query,
    Transaction
);
