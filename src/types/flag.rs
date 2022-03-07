use std::fmt::{Debug, Display};

use pyo3::{IntoPy, PyObject, ToPyObject};
use serde::{Deserialize, Serialize};

/// An transaction or posting flag.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flag {
    OKAY,
    WARNING,
    PADDING,
    SUMMARIZE,
    TRANSFER,
    CONVERSIONS,
    UNREALIZED,
    RETURNS,
    MERGING,
}

impl Serialize for Flag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let char: char = (*self).into();
        serializer.serialize_char(char)
    }
}

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Flag::try_from(s.as_ref()).unwrap_or_default())
    }
}

impl Debug for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char: char = (*self).into();
        Debug::fmt(&char, f)
    }
}
impl Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char: char = (*self).into();
        Display::fmt(&char, f)
    }
}

impl TryFrom<&str> for Flag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "*" => Ok(Self::OKAY),
            "!" => Ok(Self::WARNING),
            "P" => Ok(Self::PADDING),
            "S" => Ok(Self::SUMMARIZE),
            "T" => Ok(Self::TRANSFER),
            "C" => Ok(Self::CONVERSIONS),
            "U" => Ok(Self::UNREALIZED),
            "R" => Ok(Self::RETURNS),
            "M" => Ok(Self::MERGING),
            _ => Err(()),
        }
    }
}

impl From<Flag> for char {
    fn from(value: Flag) -> Self {
        match value {
            Flag::OKAY => '*',
            Flag::WARNING => '!',
            Flag::PADDING => 'P',
            Flag::SUMMARIZE => 'S',
            Flag::TRANSFER => 'T',
            Flag::CONVERSIONS => 'C',
            Flag::UNREALIZED => 'U',
            Flag::RETURNS => 'R',
            Flag::MERGING => 'M',
        }
    }
}

impl ToPyObject for Flag {
    fn to_object(&self, py: pyo3::Python<'_>) -> PyObject {
        Into::<char>::into(*self).to_object(py)
    }
}

impl IntoPy<PyObject> for Flag {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        self.to_object(py)
    }
}

impl Default for Flag {
    fn default() -> Self {
        Self::OKAY
    }
}
