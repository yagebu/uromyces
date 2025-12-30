use crate::errors::UroError;
use crate::types::{Filename, LineNumber};

use super::NodeGetters;
use super::convert::ConversionState;

/// An error that might occur when trying to parse a string with tree-sitter.
#[derive(Debug)]
pub enum ParsingError {
    /// Parsing timeout.
    ParsingTimedOut,
}
impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParsingTimedOut => {
                write!(f, "Parsing with tree-sitter timed out.")
            }
        }
    }
}

/// An error that occurs on converting a tree-sitter tree to Rust data structures.
#[derive(Debug)]
pub struct ConversionError {
    filename: Filename,
    line: LineNumber,
    kind: ConversionErrorKind,
}

impl ConversionError {
    pub(super) fn new(
        kind: ConversionErrorKind,
        node: &tree_sitter::Node,
        s: &ConversionState,
    ) -> Self {
        Self {
            filename: s.filename.clone(),
            line: node.line_number(),
            kind,
        }
    }
}

/// An error that occurs on converting a tree-sitter tree to Rust data structures.
#[derive(Debug)]
pub enum ConversionErrorKind {
    InvalidBookingMethod(String),
    InvalidDate(String),
    InvalidDecimal(String, String),
    InvalidDocumentFilename(String),
    UnsupportedTotalCost,
    SyntaxError(String),
}

impl std::error::Error for ConversionError {}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        type K = ConversionErrorKind;

        match &self.kind {
            K::InvalidBookingMethod(m) => write!(f, "Invalid booking method: {m}"),
            K::InvalidDate(m) => write!(f, "Invalid date: {m}"),
            K::InvalidDecimal(m, decimal_error) => {
                write!(f, "Invalid decimal number '{m}': {decimal_error}")
            }
            K::InvalidDocumentFilename(m) => write!(f, "Invalid document filename: {m}"),
            K::UnsupportedTotalCost => write!(
                f,
                "the deprecated total cost syntax '{{}}' brackets is not supported"
            ),
            K::SyntaxError(s) => {
                write!(f, "Invalid syntax: {s}")
            }
        }
    }
}

impl From<ConversionError> for UroError {
    fn from(e: ConversionError) -> Self {
        Self::new(e.to_string()).with_position(e.filename.clone(), e.line)
    }
}
