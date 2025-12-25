use std::convert::Infallible;
use std::fmt::{Debug, Display};

use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::{exceptions::PyValueError, types::PyString};
use serde::{Deserialize, Serialize, de};

/// An transaction or posting flag.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Flag(u8);

impl Flag {
    pub const OKAY: Flag = Flag(b'*');
    pub const WARNING: Flag = Flag(b'!');
    pub const PADDING: Flag = Flag(b'P');
    pub const SUMMARIZE: Flag = Flag(b'S');
    pub const TRANSFER: Flag = Flag(b'T');
    pub const CONVERSIONS: Flag = Flag(b'C');
    pub const UNREALIZED: Flag = Flag(b'U');
    pub const RETURNS: Flag = Flag(b'R');
    pub const MERGING: Flag = Flag(b'M');
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

impl Default for Flag {
    fn default() -> Self {
        Self::OKAY
    }
}

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let char = char::deserialize(deserializer)?;
        Self::try_from(char)
            .map_err(|_e| de::Error::invalid_value(de::Unexpected::Char(char), &"a valid flag"))
    }
}

impl Debug for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char: char = (*self).into();
        f.debug_tuple("Flag").field(&char).finish()
    }
}
impl Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char: char = (*self).into();
        Display::fmt(&char, f)
    }
}

impl TryFrom<u8> for Flag {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'A'..=b'Z' | b'*' | b'!' | b'&' | b'?' | b'%' | b'#' => Ok(Flag(value)),
            _ => Err(()),
        }
    }
}
impl TryFrom<char> for Flag {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Self::try_from(value as u8)
        } else {
            Err(())
        }
    }
}

impl TryFrom<&str> for Flag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let first = value.as_bytes().first();
        match first {
            Some(inner) => Self::try_from(*inner),
            _ => Err(()),
        }
    }
}

impl From<Flag> for char {
    fn from(value: Flag) -> Self {
        Self::from_u32(value.0.into()).expect("flag to be valid character")
    }
}

impl<'py> IntoPyObject<'py> for Flag {
    type Target = PyString;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Into::<char>::into(self).into_pyobject(py)
    }
}

impl<'py> FromPyObject<'_, 'py> for Flag {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let str = obj.extract::<PyBackedStr>()?;
        Self::try_from(&*str).map_err(|_e| PyValueError::new_err("Invalid flag"))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flag_print() {
        assert_eq!(format!("{}", Flag::OKAY), "*");
        assert_eq!(format!("{:?}", Flag::OKAY), "Flag('*')");
    }

    #[test]
    fn flag_from_u8() {
        assert!(Flag::try_from(b'a').is_err());
        assert!(Flag::try_from(b'.').is_err());
        assert!(Flag::try_from(b'\n').is_err());

        assert!(Flag::try_from(b'A').is_ok());
        assert!(Flag::try_from(b'P').is_ok());
        assert!(Flag::try_from(b'!').is_ok());
        assert!(Flag::try_from(b'%').is_ok());
    }

    #[test]
    fn flag_from_char() {
        assert!(Flag::try_from('a').is_err());
        assert!(Flag::try_from('.').is_err());
        assert!(Flag::try_from('\n').is_err());

        assert!(Flag::try_from('P').is_ok());
        assert!(Flag::try_from('!').is_ok());
        assert!(Flag::try_from('%').is_ok());

        let a: char = Flag::try_from('A').unwrap().into();
        assert_eq!(a, 'A');
    }

    #[test]
    fn flag_serialisation() {
        let ok_serialised = serde_json::to_string(&Flag::OKAY).unwrap();
        assert_eq!(ok_serialised, "\"*\"");
        let ok_deserialised: Flag = serde_json::from_str(&ok_serialised).unwrap();
        assert_eq!(ok_deserialised, Flag::OKAY);

        let number: Result<Flag, _> = serde_json::from_str("12");
        assert!(number.is_err());

        let long_str: Result<Flag, _> = serde_json::from_str("\"ASDF\"");
        assert!(long_str.is_err());
    }

    #[test]
    fn flag_to_char() {
        let ok: char = Flag::OKAY.into();
        assert_eq!(ok, '*');
        let padding: char = Flag::PADDING.into();
        assert_eq!(padding, 'P');
    }
}
