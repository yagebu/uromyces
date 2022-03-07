//! Convert from tree-sitter nodes to Rust types.
//!
//! The tree-sitter tree already represents the input file in a way that is quite close to the way
//! that the Rust structures are layed out. So most of the code here just has to do a simple
//! mapping, however, in some cases like numbers (simple computation) and postings (e.g. computing
//! per-unit price from total price) some more work has to be done.
//!
//! Of course, errors may occur on this conversion. Some should not really happen, like an required
//! field missing from a tree-sitter node and others, like an invalid date can simply happen due to
//! invalid input data. The latter kind should be bubbled up and will be attached to the list of
//! errors that can be presented to the user.

use tree_sitter::Node;

use super::errors::ConversionError;
use super::errors::ConversionErrorKind::{
    InvalidBookingMethod, InvalidDate, InvalidDecimal, UnsupportedTotalCost,
};
use super::node_fields;
use super::node_ids;
use super::ConversionResult;
use super::NodeGetters;
use crate::types::{
    Account, Amount, Balance, Booking, Close, Commodity, CostSpec, Currency, Custom, Date, Decimal,
    Document, EntryHeader, Event, FilePath, Flag, IncompleteAmount, Meta, MetaKeyValuePair,
    MetaValue, Note, Open, Pad, Price, Query, RawPosting, RawTransaction, TagsLinks,
};

/// The state that all conversion node handlers have access to.
pub(super) struct ConversionState<'source> {
    /// The source string.
    pub string: &'source str,
    /// The filename of the file being parsed.
    pub filename: &'source Option<FilePath>,
    /// The currently pushed metadata.
    pub pushed_meta: Meta,
    /// The currently pushed tags.
    pub pushed_tags: TagsLinks,
}

impl<'source> ConversionState<'source> {
    pub fn new(string: &'source str, filename: &'source Option<FilePath>) -> Self {
        Self {
            string,
            filename,
            pushed_meta: Vec::new(),
            pushed_tags: TagsLinks::new(),
        }
    }

    /// Get the full str contents of the node.
    fn get_str(&self, node: Node) -> &'source str {
        &self.string[node.start_byte()..node.end_byte()]
    }

    /// Get the single char of a flag node.
    fn get_flag(&self, node: Node) -> Flag {
        Flag::try_from(&self.string[node.start_byte()..node.end_byte()]).unwrap_or_default()
    }

    /// Get the contents of a string-like node.
    fn get_string(&self, node: Node) -> &'source str {
        &self.string[node.start_byte() + 1..node.end_byte() - 1]
    }

    /// Get the contents of a key-like node.
    pub fn get_key(&self, node: Node) -> &'source str {
        &self.string[node.start_byte()..node.end_byte() - 1]
    }

    /// Get the contents of a tag or link node.
    pub fn get_tag_link(&self, node: Node) -> &'source str {
        &self.string[node.start_byte() + 1..node.end_byte()]
    }
}

/// Convert from tree-sitter nodes to Rust types.
pub(super) trait TryFromNode {
    /// Parse Beancount object from its tree-sitter tree node.
    /// Should only be called on a tree node of correct type.
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self>
    where
        Self: Sized;
}

/// Convert from tree-sitter nodes to Rust types (if the conversion cannot fail).
pub(super) trait FromNode {
    /// Parse Beancount object from its tree-sitter tree node.
    /// Should only be called on a tree node of correct type.
    /// Just like `try_from_node` in the trait `TryFromNode`, but never fails.
    fn from_node(node: Node, s: &ConversionState) -> Self
    where
        Self: Sized;
}

impl FromNode for String {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "string");
        s.get_string(node).into()
    }
}

impl FromNode for Account {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert!(node.kind() == "account",);
        s.get_str(node).into()
    }
}

impl FromNode for Currency {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert!(node.kind() == "currency",);
        s.get_str(node).into()
    }
}

impl FromNode for Vec<Currency> {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "currency_list");
        node.named_children(&mut node.walk())
            .map(|n| Currency::from_node(n, s))
            .collect()
    }
}

impl TryFromNode for Date {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "date");
        Date::try_from_str(s.get_str(node))
            .map_err(|_| ConversionError::new(InvalidDate(s.get_str(node).into()), &node, s))
    }
}

impl TryFromNode for Booking {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "string");
        let method = s.get_string(node);
        Booking::try_from(method)
            .map_err(|()| ConversionError::new(InvalidBookingMethod(method.into()), &node, s))
    }
}

impl TryFromNode for Decimal {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        match node.kind_id() {
            node_ids::NUMBER => {
                let contents = s.get_str(node);
                let dec = if contents.contains(',') {
                    // TODO(perf): this currently creates an intermediate String
                    Decimal::from_str_exact(&contents.replace(',', ""))
                } else {
                    Decimal::from_str_exact(contents)
                };
                dec.map_err(|e| {
                    ConversionError::new(InvalidDecimal(contents.into(), e.to_string()), &node, s)
                })
            }
            node_ids::PAREN_NUM_EXPR => Decimal::try_from_node(node.required_child(1), s),
            node_ids::UNARY_NUM_EXPR => {
                let num = Decimal::try_from_node(node.required_child(1), s)?;
                let sign = s.get_str(node.required_child(0));
                Ok(match sign {
                    "-" => -num,
                    _ => num,
                })
            }
            node_ids::BINARY_NUM_EXPR => {
                let left = Decimal::try_from_node(node.required_child(0), s)?;
                let right = Decimal::try_from_node(node.required_child(2), s)?;
                let op = s.get_str(node.required_child(1));
                Ok(match op {
                    "+" => left + right,
                    "-" => left - right,
                    "*" => left * right,
                    _ => left / right,
                })
            }
            _ => panic!("Invalid number node."),
        }
    }
}

impl TryFromNode for CostSpec {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert!(node.kind() == "cost" || node.kind() == "total_cost");
        if node.kind_id() == node_ids::TOTAL_COST {
            return Err(ConversionError::new(UnsupportedTotalCost, &node, s));
        }
        let merge = node.child_by_field_id(node_fields::MERGE).is_some();
        let date = node
            .child_by_field_id(node_fields::DATE)
            .map(|n| Date::try_from_node(n, s))
            .transpose()?;
        let label = node
            .child_by_field_id(node_fields::STRING)
            .map(|n| String::from_node(n, s));

        let mut number_per = None;
        let mut number_total = None;
        let mut currency = None;

        if let Some(compound_amount) = node.child_by_field_id(node_fields::COMPOUND_AMOUNT) {
            number_per = compound_amount
                .child_by_field_id(node_fields::NUMBER_PER)
                .map(|m| Decimal::try_from_node(m, s))
                .transpose()?;
            number_total = compound_amount
                .child_by_field_id(node_fields::NUMBER_TOTAL)
                .map(|m| Decimal::try_from_node(m, s))
                .transpose()?;
            currency = compound_amount
                .child_by_field_id(node_fields::CURRENCY)
                .map(|m| Currency::from_node(m, s));
        }

        Ok(CostSpec {
            number_per,
            number_total,
            currency,
            date,
            label,
            merge,
        })
    }
}

impl TryFromNode for RawPosting {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        let flag = node.child_by_field_id(node_fields::FLAG);
        let units = node
            .child_by_field_id(node_fields::AMOUNT)
            .map(|n| IncompleteAmount::try_from_node(n, s))
            .transpose()?
            .unwrap_or_default();
        let price_annotation = node.child_by_field_id(node_fields::PRICE_ANNOTATION);
        let price = if let Some(price_n) = price_annotation {
            if let Some(amount_n) = price_n.child(1) {
                let price_amt = IncompleteAmount::try_from_node(amount_n, s)?;
                let total_price = price_n.kind_id() == node_ids::TOTAL_PRICE_ANNOTATION;
                Some(if total_price {
                    match (price_amt.number, units.number) {
                        (Some(price_num), Some(units_number)) => IncompleteAmount {
                            number: Some(price_num / units_number.abs()),
                            ..price_amt
                        },
                        _ => price_amt,
                    }
                } else {
                    price_amt
                })
            } else {
                Some(IncompleteAmount::default())
            }
        } else {
            None
        };
        Ok(RawPosting {
            filename: s.filename.clone(),
            line: node.line_number() + 1,
            meta: node
                .child_by_field_id(node_fields::METADATA)
                .map(|n| Meta::try_from_node(n, s))
                .transpose()?
                .unwrap_or_default(),
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            flag: flag.map(|n| s.get_flag(n)),
            units,
            price,
            cost: node
                .child_by_field_id(node_fields::COST_SPEC)
                .map(|n| CostSpec::try_from_node(n, s))
                .transpose()?,
        })
    }
}

impl TryFromNode for IncompleteAmount {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert!(node.kind() == "amount" || node.kind() == "incomplete_amount",);
        Ok(IncompleteAmount {
            number: node
                .child_by_field_id(node_fields::NUMBER)
                .map(|n| Decimal::try_from_node(n, s))
                .transpose()?,
            currency: node
                .child_by_field_id(node_fields::CURRENCY)
                .map(|n| Currency::from_node(n, s)),
        })
    }
}

impl TryFromNode for Amount {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert!(node.kind() == "amount" || node.kind() == "amount_with_tolerance",);
        Ok(Amount {
            number: Decimal::try_from_node(node.required_child_by_id(node_fields::NUMBER), s)?,
            currency: Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
        })
    }
}

impl TryFromNode for MetaValue {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        Ok(match node.kind_id() {
            node_ids::STRING => MetaValue::String(String::from_node(node, s)),
            node_ids::DATE => MetaValue::Date(Date::try_from_node(node, s)?),
            node_ids::TAG => MetaValue::Tag(s.get_tag_link(node).into()),
            node_ids::ACCOUNT => MetaValue::Account(Account::from_node(node, s)),
            node_ids::BOOL => MetaValue::Bool(s.get_str(node) == "TRUE"),
            node_ids::AMOUNT => MetaValue::Amount(Amount::try_from_node(node, s)?),
            _ => MetaValue::Number(Decimal::try_from_node(node, s)?),
        })
    }
}
impl TryFromNode for MetaKeyValuePair {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "key_value");
        Ok(MetaKeyValuePair {
            key: s.get_key(node.required_child(0)).into(),
            value: if let Some(n) = node.child(1) {
                Some(MetaValue::try_from_node(n, s)?)
            } else {
                None
            },
        })
    }
}

impl TryFromNode for Meta {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "metadata");
        let mut res = node
            .children(&mut node.walk())
            .map(|n| MetaKeyValuePair::try_from_node(n, s))
            .collect::<ConversionResult<_>>()?;
        if s.pushed_meta.is_empty() {
            Ok(res)
        } else {
            let mut meta = s.pushed_meta.clone();
            meta.append(&mut res);
            Ok(meta)
        }
    }
}

impl TryFromNode for EntryHeader {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        let mut tags = s.pushed_tags.clone();
        let mut links = TagsLinks::new();
        let tags_and_links = node.child_by_field_id(node_fields::TAGS_AND_LINKS);
        if let Some(n) = tags_and_links {
            for child in n.children(&mut n.walk()) {
                let name = s.get_tag_link(child).into();
                if child.kind_id() == node_ids::TAG {
                    tags.insert(name);
                } else {
                    links.insert(name);
                }
            }
        };
        Ok(EntryHeader {
            date: Date::try_from_node(node.required_child_by_id(node_fields::DATE), s)?,
            meta: node
                .child_by_field_id(node_fields::METADATA)
                .map(|n| Meta::try_from_node(n, s))
                .transpose()?
                .unwrap_or_default(),
            tags,
            links,
            filename: s.filename.clone(),
            line: node.line_number(),
        })
    }
}

impl TryFromNode for Balance {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "balance");
        let amt = node.required_child_by_id(node_fields::AMOUNT);
        let tol = amt.child_by_field_id(node_fields::TOLERANCE);
        Ok(Balance {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            amount: Amount::try_from_node(amt, s)?,
            tolerance: tol.map(|n| Decimal::try_from_node(n, s)).transpose()?,
        })
    }
}

impl TryFromNode for Commodity {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "commodity");
        Ok(Commodity {
            header: EntryHeader::try_from_node(node, s)?,
            currency: Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
        })
    }
}

impl TryFromNode for Close {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        Ok(Close {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
        })
    }
}

impl TryFromNode for Custom {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "custom");
        Ok(Custom {
            header: EntryHeader::try_from_node(node, s)?,
            r#type: String::from_node(node.required_child_by_id(node_fields::NAME), s),
            values: node
                .children(&mut node.walk())
                .skip(2)
                .map(|n| MetaValue::try_from_node(n, s))
                .collect::<ConversionResult<_>>()?,
        })
    }
}

impl TryFromNode for Document {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "document");
        let raw_path = String::from_node(node.required_child_by_id(node_fields::FILENAME), s);
        Ok(Document {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            // TODO: handle error
            filename: (raw_path.as_str()).try_into().unwrap(),
        })
    }
}

impl TryFromNode for Event {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "event");
        Ok(Event {
            header: EntryHeader::try_from_node(node, s)?,
            r#type: String::from_node(node.required_child_by_id(node_fields::TYPE), s),
            description: String::from_node(node.required_child_by_id(node_fields::DESCRIPTION), s),
        })
    }
}

impl TryFromNode for Note {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "note");
        Ok(Note {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            comment: String::from_node(node.required_child_by_id(node_fields::NOTE), s),
        })
    }
}

impl TryFromNode for Open {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        Ok(Open {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            currencies: node
                .child_by_field_id(node_fields::CURRENCIES)
                .map(|n| Vec::<Currency>::from_node(n, s))
                .unwrap_or_default(),
            booking: node
                .child_by_field_id(node_fields::BOOKING)
                .map(|n| Booking::try_from_node(n, s))
                .transpose()?,
        })
    }
}

impl TryFromNode for Pad {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "pad");
        Ok(Pad {
            header: EntryHeader::try_from_node(node, s)?,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            source_account: Account::from_node(
                node.required_child_by_id(node_fields::FROM_ACCOUNT),
                s,
            ),
        })
    }
}

impl TryFromNode for Price {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "price");
        Ok(Price {
            header: EntryHeader::try_from_node(node, s)?,
            currency: Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
            amount: Amount::try_from_node(node.required_child_by_id(node_fields::AMOUNT), s)?,
        })
    }
}

impl TryFromNode for RawTransaction {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "transaction");
        Ok(RawTransaction {
            header: EntryHeader::try_from_node(node, s)?,
            flag: s.get_flag(node.required_child_by_id(node_fields::FLAG)),
            payee: node
                .child_by_field_id(node_fields::PAYEE)
                .map(|n| String::from_node(n, s)),
            narration: node
                .child_by_field_id(node_fields::NARRATION)
                .map(|n| String::from_node(n, s)),
            postings: node
                .child_by_field_id(node_fields::POSTINGS)
                .map(|n| {
                    n.children(&mut n.walk())
                        .map(|p| RawPosting::try_from_node(p, s))
                        .collect::<ConversionResult<_>>()
                })
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl TryFromNode for Query {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "query");
        Ok(Query {
            header: EntryHeader::try_from_node(node, s)?,
            name: String::from_node(node.required_child_by_id(node_fields::NAME), s),
            query_string: String::from_node(node.required_child_by_id(node_fields::QUERY), s),
        })
    }
}
