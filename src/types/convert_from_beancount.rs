//! Conversion of `beancount.core.data` entries into uromyces entries.
use pyo3::exceptions::PyValueError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyString, PyType};

use crate::types::{
    Balance, Booking, BoxStr, Close, Commodity, Currency, Custom, CustomValue, Document, Entry,
    Event, Flag, Note, Open, Pad, Posting, Price, Query, TagsLinks, Transaction,
};

pub(crate) trait ConvertFromBeancount
where
    Self: Sized,
{
    /// Try to convert an object from its matching Beancount type.
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self>;
}
impl<T: ConvertFromBeancount> ConvertFromBeancount for Option<T> {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(if entry.is_none() {
            None
        } else {
            Some(T::convert_from_beancount(entry)?)
        })
    }
}
impl<T: ConvertFromBeancount> ConvertFromBeancount for Vec<T> {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        entry
            .try_iter()?
            .map(|item| T::convert_from_beancount(&item?))
            .collect::<PyResult<_>>()
    }
}

impl ConvertFromBeancount for Entry {
    /// Try to convert a Beancount or uromyces entry or into a uromyces [`Entry`].
    ///
    /// Beancount entries are converted into the `Entry` variant, otherwise we try to extract and
    /// [`Entry`].
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        let ty = entry.get_type();

        macro_rules! try_convert {
            ($class_name:literal, $variant:ident) => {{
                static CACHE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
                let cls = CACHE.import(py, "beancount.core.data", $class_name)?;
                if ty.is(cls) {
                    return Ok(Entry::$variant($variant::convert_from_beancount(entry)?));
                }
            }};
        }

        try_convert!("Transaction", Transaction);
        try_convert!("Price", Price);
        try_convert!("Document", Document);
        try_convert!("Balance", Balance);
        try_convert!("Open", Open);
        try_convert!("Close", Close);
        try_convert!("Pad", Pad);
        try_convert!("Note", Note);
        try_convert!("Commodity", Commodity);
        try_convert!("Event", Event);
        try_convert!("Custom", Custom);
        try_convert!("Query", Query);

        entry.extract()
    }
}

impl ConvertFromBeancount for Transaction {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Transaction::new(
            entry.getattr(intern!(py, "meta"))?.extract()?,
            entry.getattr(intern!(py, "date"))?.extract()?,
            entry.getattr(intern!(py, "tags"))?.extract()?,
            entry.getattr(intern!(py, "links"))?.extract()?,
            entry
                .getattr(intern!(py, "flag"))?
                .extract::<Option<Flag>>()?
                .unwrap_or(Flag::OKAY),
            entry.getattr(intern!(py, "payee"))?.extract()?,
            entry
                .getattr(intern!(py, "narration"))?
                .extract::<BoxStr>()?,
            Vec::<Posting>::convert_from_beancount(&entry.getattr(intern!(py, "postings"))?)?,
        ))
    }
}
impl ConvertFromBeancount for Posting {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Posting {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            units: entry.getattr(intern!(py, "units"))?.extract()?,
            price: entry.getattr(intern!(py, "price"))?.extract()?,
            cost: entry.getattr(intern!(py, "cost"))?.extract()?,
            flag: entry.getattr(intern!(py, "flag"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Balance {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Balance {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            amount: entry.getattr(intern!(py, "amount"))?.extract()?,
            tolerance: entry.getattr(intern!(py, "tolerance"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Close {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Close {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            account: entry.getattr(intern!(py, "account"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Commodity {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Commodity {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            currency: entry.getattr(intern!(py, "currency"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Custom {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Custom {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            r#type: entry.getattr(intern!(py, "type"))?.extract()?,
            values: Vec::<CustomValue>::convert_from_beancount(
                &entry.getattr(intern!(py, "values"))?,
            )?,
        })
    }
}
impl ConvertFromBeancount for CustomValue {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        let dtype = entry.getattr(intern!(py, "dtype"))?;
        CustomValue::__new__(py, entry.getattr(intern!(py, "value"))?.extract()?, &dtype)
    }
}
impl ConvertFromBeancount for Document {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Document {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: entry.getattr(intern!(py, "tags"))?.extract()?,
            links: entry.getattr(intern!(py, "links"))?.extract()?,
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            filename: entry.getattr(intern!(py, "filename"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Event {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Event {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            r#type: entry.getattr(intern!(py, "type"))?.extract()?,
            description: entry.getattr(intern!(py, "description"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Note {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Note {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            comment: entry.getattr(intern!(py, "comment"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Booking {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        let py_value = entry.getattr(intern!(py, "value"))?;
        let value = py_value.cast::<PyString>()?.to_str()?;
        Booking::try_from(value)
            .map_err(|()| PyValueError::new_err(format!("Invalid booking value: {value}")))
    }
}
impl ConvertFromBeancount for Open {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Open {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            currencies: entry
                .getattr(intern!(py, "currencies"))?
                .extract::<Option<Vec<Currency>>>()?
                .unwrap_or_default(),
            booking: Option::<Booking>::convert_from_beancount(
                &entry.getattr(intern!(py, "booking"))?,
            )?,
        })
    }
}
impl ConvertFromBeancount for Pad {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Pad {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            account: entry.getattr(intern!(py, "account"))?.extract()?,
            source_account: entry.getattr(intern!(py, "source_account"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Price {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Price {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            currency: entry.getattr(intern!(py, "currency"))?.extract()?,
            amount: entry.getattr(intern!(py, "amount"))?.extract()?,
        })
    }
}
impl ConvertFromBeancount for Query {
    fn convert_from_beancount(entry: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = entry.py();
        Ok(Query {
            meta: entry.getattr(intern!(py, "meta"))?.extract()?,
            date: entry.getattr(intern!(py, "date"))?.extract()?,
            tags: TagsLinks::default(),
            links: TagsLinks::default(),
            name: entry.getattr(intern!(py, "name"))?.extract()?,
            query_string: entry.getattr(intern!(py, "query_string"))?.extract()?,
        })
    }
}
