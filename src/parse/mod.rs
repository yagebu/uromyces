//! Parse a string to a raw list of Beancount entries.
//!
//! This uses a `tree_sitter` parser to parse the file to an AST and then constructs Beancount
//! directives from that AST.

use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser, Tree};

use self::convert::{ConversionState, FromNode, TryFromNode};
use self::errors::ConversionErrorKind::SyntaxError;
use self::errors::{ConversionError, ParsingError};
use crate::errors::UroError;
use crate::types::{
    Balance, Close, Commodity, Custom, Document, Event, FilePath, LineNumber, MetaKeyValuePair,
    Note, Open, Pad, Price, Query, RawDirective, RawEntry, RawTransaction,
};

mod convert;
mod errors;
mod node_fields;
mod node_ids;
#[cfg(test)]
mod tests;

extern "C" {
    fn tree_sitter_beancount() -> Language;
}

/// Get the Beancount tree-sitter grammar.
fn get_beancount_language() -> Language {
    unsafe { tree_sitter_beancount() }
}

/// Initialise a Beancount language `tree_sitter` parser.
///
/// # Panics
///
/// Since we cannot do a lot in uromyces without the parser, panic if the Beancount language cannot
/// be loaded due to a version mismatch.
fn init_parser() -> Parser {
    let mut parser = Parser::new();
    let language = get_beancount_language();
    parser
        .set_language(&language)
        .expect("tree-sitter language and library version to match");
    parser
}

/// The tree-sitter `Tree` produced by parsing a string.
///
/// Since the `Tree` itself only contains the node positions, we also need
/// to keep the string around to be able to obtain the node values.
pub struct ParsedTree<'source> {
    /// A tree-sitter tree.
    tree: Tree,
    /// The parsed string.
    string: &'source str,
}

/// Parse a string to a tree-sitter Tree.
///
/// # Errors
///
/// `ParsingError` if parsing times out.
pub fn string_to_tree(string: &str) -> Result<ParsedTree, ParsingError> {
    let mut parser = init_parser();
    parser
        .parse(string, None)
        .map(|tree| ParsedTree { tree, string })
        .ok_or(ParsingError::ParsingTimedOut)
}

/// The raw result of parsing the code of a single Beancount file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ParsedFile {
    /// The (raw) entries in the file.
    pub entries: Vec<RawEntry>,
    /// Errors encountered on converting the parse tree to `ParseResult`.
    pub errors: Vec<UroError>,
    /// The directives (options, includes and plugins) in the file.
    pub directives: Vec<RawDirective>,
}

impl ParsedFile {
    /// Create an empty `ParseResult` with a given entry capacity.
    fn with_entries_capacity(size: usize) -> Self {
        Self {
            entries: Vec::with_capacity(size),
            ..Self::default()
        }
    }

    /// Create an empty `ParseResult` for a single error.
    #[must_use]
    pub fn from_error(error: UroError) -> Self {
        Self {
            errors: vec![error],
            ..Self::default()
        }
    }
}

type ConversionResult<T> = Result<T, ConversionError>;

trait NodeGetters {
    /// Obtain the child at the given index (or error if it does not exist).
    fn required_child(&self, i: usize) -> Node;
    /// Obtain the child with the given field id (or error if it does not exist).
    fn required_child_by_id(&self, id: u16) -> Node;
    /// Get the starting line number of the node.
    fn line_number(&self) -> LineNumber;
}

impl NodeGetters for Node<'_> {
    fn required_child(&self, i: usize) -> Node {
        self.child(i)
            .expect("required node child at given to exist")
    }
    fn required_child_by_id(&self, id: u16) -> Node {
        self.child_by_field_id(id)
            .expect("required node child for given field to exist")
    }
    fn line_number(&self) -> LineNumber {
        (self.start_position().row + 1)
            .try_into()
            .expect("line number to be small enough")
    }
}

/// Parse a string to Beancount entries.
#[must_use]
#[allow(clippy::module_name_repetitions)]
pub fn parse_string(s: &str, filename: &Option<FilePath>) -> ParsedFile {
    match string_to_tree(s) {
        Ok(tree) => convert_syntax_tree(&tree, filename),
        Err(err) => {
            let e = UroError::new(format!("Parsing file failed with an error: {err}"));
            ParsedFile::from_error(match filename {
                Some(p) => e.with_filename(p),
                None => e,
            })
        }
    }
}

/// Convert a tree-sitter AST to a list of (unbooked) Beancount entries.
///
/// This, like the parser before it, operates on a single file. The results from multiple files
/// can be combined in a subsequent step to obtain a single list of entries ready for booking.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn convert_syntax_tree(parsed_tree: &ParsedTree, filename: &Option<FilePath>) -> ParsedFile {
    let state = &mut ConversionState::new(parsed_tree.string, filename.as_ref());
    // this is the cursor we use to iterate over all entries.
    let root_node = parsed_tree.tree.root_node();
    let mut result = ParsedFile::with_entries_capacity(root_node.child_count());

    for node in root_node.children(&mut root_node.walk()) {
        if node.has_error() {
            let err = ConversionError::new(SyntaxError(node.to_sexp()), &node, state);
            result.errors.push(err.into());
            continue;
        }

        let res: Result<(), UroError> = (|| {
            match node.kind_id() {
                node_ids::TRANSACTION => {
                    result
                        .entries
                        .push(RawTransaction::try_from_node(node, state)?.into());
                }
                node_ids::PRICE => {
                    result
                        .entries
                        .push(Price::try_from_node(node, state)?.into());
                }
                node_ids::BALANCE => {
                    result
                        .entries
                        .push(Balance::try_from_node(node, state)?.into());
                }
                node_ids::CLOSE => {
                    result
                        .entries
                        .push(Close::try_from_node(node, state)?.into());
                }
                node_ids::COMMODITY => {
                    result
                        .entries
                        .push(Commodity::try_from_node(node, state)?.into());
                }
                node_ids::CUSTOM => {
                    result
                        .entries
                        .push(Custom::try_from_node(node, state)?.into());
                }
                node_ids::DOCUMENT => {
                    result
                        .entries
                        .push(Document::try_from_node(node, state)?.into());
                }
                node_ids::EVENT => {
                    result
                        .entries
                        .push(Event::try_from_node(node, state)?.into());
                }
                node_ids::NOTE => {
                    result
                        .entries
                        .push(Note::try_from_node(node, state)?.into());
                }
                node_ids::OPEN => {
                    result
                        .entries
                        .push(Open::try_from_node(node, state)?.into());
                }
                node_ids::PAD => {
                    result.entries.push(Pad::try_from_node(node, state)?.into());
                }
                node_ids::QUERY => {
                    result
                        .entries
                        .push(Query::try_from_node(node, state)?.into());
                }
                node_ids::OPTION => {
                    result.directives.push(RawDirective::Option {
                        filename: filename.clone(),
                        line: node.line_number(),
                        key: String::from_node(node.required_child(1), state),
                        value: String::from_node(node.required_child(2), state),
                    });
                }
                node_ids::INCLUDE => {
                    result.directives.push(RawDirective::Include {
                        pattern: String::from_node(node.required_child(1), state),
                    });
                }
                node_ids::PLUGIN => {
                    result.directives.push(RawDirective::Plugin {
                        name: String::from_node(node.required_child(1), state),
                        config: node.child(2).map(|n| String::from_node(n, state)),
                    });
                }
                node_ids::PUSHMETA => {
                    let key_value = MetaKeyValuePair::try_from_node(node.required_child(1), state)?;
                    state.pushed_meta.push(key_value);
                }
                node_ids::PUSHTAG => {
                    let tag = state.get_tag_link(node.required_child(1));
                    state.pushed_tags.insert(tag.into());
                }
                node_ids::POPMETA => {
                    let key = state.get_key(node.required_child(1));
                    state.pushed_meta.remove(key);
                }
                node_ids::POPTAG => {
                    let tag = state.get_tag_link(node.required_child(1));
                    state.pushed_tags.remove(tag);
                }
                _ => {
                    println!("Unknown node kind: {}", node.kind());
                }
            };
            Ok(())
        })();
        if let Err(err) = res {
            result.errors.push(err);
        };
    }

    result
}
