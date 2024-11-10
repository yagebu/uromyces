use pyo3::{prelude::*, types::PyString};
use serde::{Deserialize, Serialize};

/// The various booking methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(frozen, module = "uromyces", eq, eq_int)]
pub enum Booking {
    #[pyo3(name = "STRICT")]
    #[default]
    Strict,
    #[pyo3(name = "NONE")]
    None,
    #[pyo3(name = "AVERAGE")]
    Average,
    #[pyo3(name = "FIFO")]
    Fifo,
    #[pyo3(name = "HIFO")]
    Hifo,
    #[pyo3(name = "LIFO")]
    Lifo,
}

#[pymethods]
impl Booking {
    // It needs to be passed by ref for pyo3
    #[getter]
    fn value<'py>(&self, py: Python<'py>) -> &Bound<'py, PyString> {
        match self {
            Self::Strict => pyo3::intern!(py, "STRICT"),
            Self::None => pyo3::intern!(py, "NONE"),
            Self::Average => pyo3::intern!(py, "AVERAGE"),
            Self::Fifo => pyo3::intern!(py, "FIFO"),
            Self::Hifo => pyo3::intern!(py, "HIFO"),
            Self::Lifo => pyo3::intern!(py, "LIFO"),
        }
    }
}

impl TryFrom<&str> for Booking {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NONE" => Ok(Self::None),
            "AVERAGE" => Ok(Self::Average),
            "FIFO" => Ok(Self::Fifo),
            "HIFO" => Ok(Self::Hifo),
            "LIFO" => Ok(Self::Lifo),
            "STRICT" => Ok(Self::Strict),
            _ => Err(()),
        }
    }
}
