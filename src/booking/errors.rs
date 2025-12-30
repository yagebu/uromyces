use crate::types::{Filename, LineNumber, RawPosting};

/// An error that occurs during interpolation or booking.
#[derive(Debug)]
pub struct BookingError {
    filename: Filename,
    line: LineNumber,
    kind: BookingErrorKind,
}

#[derive(Debug)]
pub(super) enum BookingErrorKind {
    // Currency resolution and grouping
    UnresolvedUnitsCurrency,
    UnresolvedCostCurrency,
    UnresolvedPriceCurrency,
    MultipleAutoPostings,
    // Closing of positions
    InsufficientLots,
    NoMatchesForReduction,
    UnsupportedAverageBooking,
    AmbiguousMatches,
    // Interpolation
    TooManyMissingNumbers,
    MissingAmountNumber,
    MissingCostNumber,
}

impl BookingErrorKind {
    pub(super) fn with_posting(self, posting: &RawPosting) -> BookingError {
        BookingError {
            filename: posting.filename.clone(),
            line: posting.line,
            kind: self,
        }
    }
}

impl std::error::Error for BookingError {}

impl std::fmt::Display for BookingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        type T = BookingErrorKind;

        match self.kind {
            T::UnresolvedUnitsCurrency => write!(f, "Unresolved units currency"),
            T::UnresolvedCostCurrency => write!(f, "Unresolved cost currency"),
            T::UnresolvedPriceCurrency => write!(f, "Unresolved price currency"),
            T::MultipleAutoPostings => write!(f, "There can be at most one auto posting"),
            T::InsufficientLots => write!(f, "Not enough lots in inventory to reduce position"),
            T::NoMatchesForReduction => {
                write!(f, "No matching lots in inventory to reduce position")
            }
            T::UnsupportedAverageBooking => {
                write!(f, "The AVERAGE booking method is not supported")
            }
            T::AmbiguousMatches => write!(f, "Ambiguous matches"),
            T::TooManyMissingNumbers => write!(f, "Too many missing numbers in transaction"),
            T::MissingAmountNumber => write!(f, "Amount is missing a number"),
            T::MissingCostNumber => write!(f, "Cost is missing a number"),
        }
    }
}

impl From<BookingError> for crate::errors::UroError {
    fn from(e: BookingError) -> Self {
        Self::new(e.to_string()).with_position(e.filename.clone(), e.line)
    }
}
