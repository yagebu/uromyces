//! The data types that are used for handling Beancount data.
//!
//! We both have some base data types as well as types for the different kinds of Beancount entry.
//!
//! To be able to apply various optimisations and properly distinguish between them, basic
//! string-like types like [`Currency`] and [`Account`] each have their own wrapper type. With
//! their help, we can use string interners and easily make specific methods (like getting the
//! parent for an account) available.
//!
//! All of these data types can be serialised with `serde`.
//!
//! ## Entries
//!
//! Entries are the central composite data structure for transactions, documents and the other
//! types of Beancount input data.
//!
//! ## Base data types
//!
//! - [`Decimal`] - all numbers that represent some financial value.
// - TODO: docs

#![allow(clippy::unsafe_derive_deserialize)]

use std::fmt::Debug;

use pyo3::prelude::*;
use pyo3::types::PyDict;
pub(crate) use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

mod account;
mod amount;
mod booking;
mod cost;
mod currency;
mod date;
mod flag;
mod paths;
mod tags_links;

pub use account::{Account, RootAccounts};
pub use amount::{Amount, IncompleteAmount};
pub use booking::Booking;
pub use cost::{Cost, CostSpec};
pub use currency::Currency;
pub use date::Date;
pub use flag::Flag;
pub use paths::FilePath;
pub use tags_links::TagsLinks;

use crate::py_bindings::decimal_to_py;

/// The type to use for line numbers in file positions.
pub type LineNumber = u32;

/// A raw Beancount directive (option, plugin, or include).
#[derive(Debug)]
pub enum RawDirective {
    /// A raw Beancount option.
    Option {
        filename: Option<FilePath>,
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
pub struct Plugin {
    pub name: String,
    pub config: Option<String>,
}

/// Possible metadata values (this is also used for custom entries).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetaValue {
    String(String),
    Tag(String),
    Date(Date),
    Account(Account),
    Bool(bool),
    Amount(Amount),
    Number(Decimal),
}

impl ToPyObject for MetaValue {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        match self {
            Self::String(v) | Self::Tag(v) => v.to_object(py),
            Self::Date(v) => v.to_object(py),
            Self::Account(v) => v.to_object(py),
            Self::Bool(v) => v.to_object(py),
            Self::Amount(v) => v.clone().into_py(py),
            Self::Number(v) => decimal_to_py(py, *v).unwrap(),
        }
    }
}

/// A single key-value pair in metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaKeyValuePair {
    pub key: String,
    pub value: Option<MetaValue>,
}

/// Metadata, a list of key-value pairs.
pub type Meta = Vec<MetaKeyValuePair>;

/// The "entry header", the data which all entries carry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryHeader {
    /// Entry date.
    pub date: Date,
    /// Entry metadata.
    pub meta: Meta,
    /// Tags of the entry.
    pub tags: TagsLinks,
    /// Links of the entry.
    pub links: TagsLinks,
    /// The filename.
    pub filename: Option<FilePath>,
    /// The 1-based line number.
    pub line: LineNumber,
}

impl EntryHeader {
    /// Create a new entry header (with empty metadata, tags and links).
    #[must_use]
    pub fn new(date: Date, filename: Option<FilePath>, line: LineNumber) -> Self {
        Self {
            date,
            meta: Meta::default(),
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            filename,
            line,
        }
    }

    /// Convert this to a Python dictionary like the `meta` attribute of Beancount entries.
    fn to_py_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let meta = PyDict::new(py);
        meta.set_item(pyo3::intern!(py, "filename"), &self.filename)?;
        meta.set_item(pyo3::intern!(py, "lineno"), self.line)?;
        for kv in &self.meta {
            meta.set_item(&kv.key, kv.value.to_object(py))?;
        }
        Ok(meta.into())
    }
}

/// A balance entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Balance {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub amount: Amount,
    // TODO #[pyo3(get)]
    pub tolerance: Option<Decimal>,
}

/// An account close entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Close {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
}

/// A commodity entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Commodity {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub currency: Currency,
}

/// A custom entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Custom {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub r#type: String,
    // TODO pyo3 get
    pub values: Vec<MetaValue>,
}

/// An document entry for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Document {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub filename: FilePath,
}

/// An event for an account.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Event {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub r#type: String,
    #[pyo3(get)]
    pub description: String,
}

/// An account open entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Open {
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
#[pyclass(frozen, module = "uromyces")]
pub struct Note {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub comment: String,
}

/// A pad entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Pad {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub account: Account,
    #[pyo3(get)]
    pub source_account: Account,
}

/// A price entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Price {
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPosting {
    /// The filename.
    pub filename: Option<FilePath>,
    /// The 1-based line number.
    pub line: LineNumber,
    pub meta: Meta,

    pub account: Account,
    pub flag: Option<Flag>,
    pub units: IncompleteAmount,
    pub price: Option<IncompleteAmount>,
    pub cost: Option<CostSpec>,
}

/// A fully booked posting.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Posting {
    /// The filename.
    pub filename: Option<FilePath>,
    /// The 1-based line number.
    pub line: LineNumber,
    pub meta: Meta,

    pub account: Account,
    pub flag: Option<Flag>,
    pub units: Amount,
    pub price: Option<Amount>,
    pub cost: Option<Cost>,
}

impl Posting {
    #[must_use]
    pub(crate) fn new_simple(account: Account, units: Amount) -> Self {
        Self {
            flag: None,
            filename: None,
            line: 0,
            account,
            units,
            cost: None,
            price: None,
            meta: Vec::new(),
        }
    }
}

/// A transaction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Transaction {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub flag: Flag,
    #[pyo3(get)]
    pub payee: Option<String>,
    #[pyo3(get)]
    pub narration: Option<String>,
    #[pyo3(get)]
    pub postings: Vec<Posting>,
}

/// A raw transaction.
///
/// After parsing, parts of the transaction might still be missing and will
/// only be inferred during booking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTransaction {
    pub header: EntryHeader,
    pub flag: Flag,
    pub payee: Option<String>,
    pub narration: Option<String>,
    pub postings: Vec<RawPosting>,
}

/// A query entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces")]
pub struct Query {
    pub header: EntryHeader,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub query_string: String,
}

/// The Beancount entries (raw, after parsing).
#[derive(Clone, Debug, PartialEq, Eq)]
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
    Transaction(RawTransaction),
    Query(Query),
}

/// The Beancount entries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "t")]
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
    Transaction(Transaction),
    Query(Query),
}

/// Since all the entry types need the same additional getter functions, this short macro provides
/// them.
macro_rules! pymethods_for_entry {
    ($a:ident) => {
        #[pymethods]
        impl $a {
            #[getter]
            fn date(&self) -> Date {
                self.header.date
            }
            #[getter]
            fn links(&self) -> &TagsLinks {
                &self.header.links
            }
            #[getter]
            fn tags(&self) -> &TagsLinks {
                &self.header.tags
            }
            #[getter]
            fn meta(&self, py: Python) -> PyResult<Py<PyDict>> {
                self.header.to_py_dict(py)
            }
        }
    };
}

// We need this newtype wrapper to define a pyclass for rust_decimal::Decimal
#[pyclass(frozen, module = "uromyces", name = "Decimal")]
pub(crate) struct DecimalPyWrapper(Decimal);

#[pymethods]
impl Posting {
    #[getter]
    fn account(&self, py: Python) -> PyObject {
        self.account.to_object(py)
    }
    #[getter]
    fn flag(&self) -> Option<Flag> {
        self.flag
    }
    #[getter]
    fn meta(&self, py: Python) -> PyResult<Py<PyDict>> {
        let meta = PyDict::new(py);
        meta.set_item(pyo3::intern!(py, "filename"), &self.filename)?;
        meta.set_item(pyo3::intern!(py, "lineno"), self.line)?;
        for kv in &self.meta {
            meta.set_item(&kv.key, kv.value.to_object(py))?;
        }
        Ok(meta.into())
    }
    #[getter]
    fn units(&self) -> Amount {
        self.units.clone()
    }
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

impl IntoPy<PyObject> for Entry {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        match self {
            Self::Balance(e) => e.into_py(py),
            Self::Close(e) => e.into_py(py),
            Self::Commodity(e) => e.into_py(py),
            Self::Custom(e) => e.into_py(py),
            Self::Document(e) => e.into_py(py),
            Self::Event(e) => e.into_py(py),
            Self::Note(e) => e.into_py(py),
            Self::Open(e) => e.into_py(py),
            Self::Pad(e) => e.into_py(py),
            Self::Price(e) => e.into_py(py),
            Self::Query(e) => e.into_py(py),
            Self::Transaction(e) => e.into_py(py),
        }
    }
}

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
            Self::Transaction(e) => &e.header,
            Self::Query(e) => &e.header,
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
        self.sort_key().partial_cmp(&other.sort_key())
    }
}

impl Ord for RawEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.sort_key().partial_cmp(&other.sort_key())
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
