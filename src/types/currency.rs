use std::fmt::{Debug, Display};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::interned_string::InternedString;

/// A currency name.
///
/// This is a newtype wrapper so that we can transparently swap out the inner type
/// for a more fitting String-like type, make it immutable and avoid mixing them up with
/// other strings like account names.
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
pub struct Currency(InternedString);

impl Debug for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Currency").field(&self.0).finish()
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
impl PartialEq<str> for Currency {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}
