use pyo3::pyclass;
use serde::{Deserialize, Serialize};

/// The various booking methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass]
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
    #[pyo3(name = "LIFO")]
    Lifo,
}

impl TryFrom<&str> for Booking {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NONE" => Ok(Booking::None),
            "AVERAGE" => Ok(Booking::Average),
            "LIFO" => Ok(Booking::Lifo),
            "FIFO" => Ok(Booking::Fifo),
            "STRICT" => Ok(Booking::Strict),
            _ => Err(()),
        }
    }
}
