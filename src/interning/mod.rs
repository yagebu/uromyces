//! Interning for strings in uromyces.
//!
//! Each interned-string type (defined via [`define_interned_string!`]) deduplicates repeated string
//! values into a single shared allocation, so that equality/hashing can be done in O(1) by
//! comparing pointers. Every type gets its own dedicated pool rather than sharing one global pool:
//! `Account`, `Currency` and path values have different validity rules.
//!
//! Interned strings are leaked: a pool holds an [`InternedValue`] for every distinct value it has
//! ever seen, and nothing is ever removed. The memory cost is bounded by the number of distinct
//! strings ever interned, which should be the right trade here: account, currency and path
//! vocabularies come from ledger files and are small and stable.
//!
//! Each [`InternedValue`] also contains a lazily-built Python string so converting an interned
//! string to Python is just an incref of a cached object.

use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::LazyLock;

use foldhash::fast::RandomState;
use papaya::HashMap;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyString;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The canonical, leaked allocation for an string value.
pub struct InternedValue {
    /// The string itself.
    value: &'static str,
    /// The Python `str` for value, built on first conversion to Python and then reused.
    py_string: PyOnceLock<Py<PyString>>,
}

impl<'py> IntoPyObject<'py> for &InternedValue {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self
            .py_string
            .get_or_init(py, || PyString::new(py, self.value).unbind())
            .bind(py)
            .clone())
    }
}

/// The table backing a single type of interned strings.
struct Pool {
    map: HashMap<&'static str, &'static InternedValue, RandomState>,
}

impl Pool {
    fn new(capacity: usize) -> Self {
        let map = HashMap::builder()
            .capacity(capacity)
            .hasher(RandomState::default())
            .build();
        Pool { map }
    }

    /// Look up or insert a string, returning the canonical shared value.
    fn intern(&self, s: &str) -> &'static InternedValue {
        let pool = self.map.pin();
        if let Some(existing) = pool.get(s) {
            return existing;
        }
        // Not present, so leak a copy and race to install it. If another thread got there first we
        // adopt its copy and abandon ours.
        let leaked = Box::leak(Box::from(s));
        let value = Box::leak(Box::new(InternedValue {
            value: leaked,
            py_string: PyOnceLock::new(),
        }));
        pool.get_or_insert_with(leaked, || value)
    }
}

/// Deserializes a string straight into an intern pool, without an intermediate [`String`].
struct InternVisitor<T>(fn(&str) -> T);

impl<T> Visitor<'_> for InternVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok((self.0)(v))
    }
}

/// Defines an interned-string newtype with its own dedicated intern pool.
///
/// The pool is initialised to a certain realistic capacity to avoid costly resizes.
macro_rules! define_interned_string {
    ($(#[$meta:meta])* $name:ident, $pool:ident, capacity = $capacity:expr) => {
        static $pool: LazyLock<Pool> = LazyLock::new(|| Pool::new($capacity));

        $(#[$meta])*
        #[derive(Clone, Copy)]
        pub struct $name(&'static InternedValue);

        impl $name {
            pub fn new(s: &str) -> Self {
                Self($pool.intern(s))
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self.0, other.0)
            }
        }
        impl Eq for $name {}
        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                std::ptr::hash(self.0, state);
            }
        }
        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.value.cmp(other.0.value)
            }
        }
        impl Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Debug::fmt(self.0.value, f)
            }
        }
        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self.0.value, f)
            }
        }
        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.0.value
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.0.value)
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_str(InternVisitor(Self::new))
            }
        }
        impl<'py> IntoPyObject<'py> for &$name {
            type Target = PyString;
            type Output = Bound<'py, Self::Target>;
            type Error = Infallible;

            fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
                self.0.into_pyobject(py)
            }
        }
        impl<'py> FromPyObject<'_, 'py> for $name {
            type Error = PyErr;

            fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
                Ok($name::new(obj.cast::<PyString>()?.to_str()?))
            }
        }
    };
}

define_interned_string!(
    /// An interned account-name string.
    AccountInternedString,
    ACCOUNT_POOL,
    capacity = 2048
);

define_interned_string!(
    /// An interned currency-code string.
    ///
    /// Currency vocabularies are much smaller than account vocabularies in practice, so this
    /// pool starts out considerably smaller.
    CurrencyInternedString,
    CURRENCY_POOL,
    capacity = 512
);

define_interned_string!(
    /// An interned file path/filename string.
    ///
    /// Approximately one filename per included file, so this pool stays far smaller than the
    /// account one.
    FilenameInternedString,
    FILENAME_POOL,
    capacity = 512
);

#[cfg(test)]
mod tests {
    use hashbrown::HashSet;

    use super::*;

    #[test]
    fn equal_content_shares_allocation() {
        let a = AccountInternedString::new("Assets:Cash");
        let b = AccountInternedString::new("Assets:Cash");
        assert_eq!(a, b);
        assert_eq!(a.as_ptr(), b.as_ptr());
    }

    #[test]
    fn interning_leaks_and_survives_dropping_every_value() {
        let ptr = {
            let a = AccountInternedString::new("Assets:Temp");
            a.as_ptr()
        };
        let b = AccountInternedString::new("Assets:Temp");
        assert_eq!(b.as_ptr(), ptr);
    }

    #[test]
    fn ord_is_content_based() {
        let a = AccountInternedString::new("b");
        let b = AccountInternedString::new("a");
        assert!(b < a);
    }

    #[test]
    fn different_types_use_different_pools() {
        let account = AccountInternedString::new("USD");
        let currency = CurrencyInternedString::new("USD");
        assert_ne!(account.as_ptr(), currency.as_ptr());
    }

    #[test]
    fn deserializes() {
        for (json, expected) in [
            (r#""Assets:Cash""#, "Assets:Cash"),
            (r#""Assets:Caf\u00e9""#, "Assets:Café"),
        ] {
            let deserialized: AccountInternedString = serde_json::from_str(json).unwrap();
            let exp = AccountInternedString::new(expected);
            assert_eq!(deserialized, exp);
            assert_eq!(deserialized.as_ptr(), exp.as_ptr());
        }
    }

    #[test]
    fn concurrent_interning_agrees_on_one_allocation() {
        const THREADS: usize = 8;
        for round in 0..128 {
            let name = format!("Assets:Race:{round}");
            let pointers = std::thread::scope(|scope| {
                let threads = (0..THREADS)
                    .map(|_| {
                        let name = name.as_str();
                        scope.spawn(move || {
                            (0..64)
                                .map(|_| {
                                    let s = AccountInternedString::new(name);
                                    s.as_ptr() as usize
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>();
                threads
                    .into_iter()
                    .flat_map(|t| t.join().unwrap())
                    .collect::<HashSet<_>>()
            });
            assert_eq!(pointers.len(), 1, "round {round} saw {pointers:?}");
        }
    }
}
