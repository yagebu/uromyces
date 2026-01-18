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

use super::ConversionResult;
use super::NodeGetters;
use super::errors::ConversionError;
use super::errors::ConversionErrorKind::{
    DivisionFailed, InternalError, InvalidBookingMethod, InvalidDate, InvalidDecimal,
    InvalidDocumentFilename, UnsupportedTotalCost,
};
use super::node_fields;
use super::node_ids;
use crate::types::{
    AbsoluteUTF8Path, Account, Amount, Balance, Booking, BoxStr, Close, Commodity, CostLabel,
    CostSpec, Currency, Custom, CustomValue, Date, Decimal, Document, EntryMeta, Event, Filename,
    Flag, Meta, MetaKeyValuePair, MetaValue, Note, Open, Pad, Price, Query, RawAmount, RawPosting,
    RawTransaction, TagsLinks,
};

/// The state that all conversion node handlers have access to.
pub(super) struct ConversionState<'source> {
    /// The source string.
    pub string: &'source str,
    /// The filename of the file being parsed.
    pub filename: &'source Filename,
    /// The currently pushed metadata.
    pub pushed_meta: Meta,
    /// The currently pushed tags.
    pub pushed_tags: TagsLinks,
}

impl<'source> ConversionState<'source> {
    pub fn new(string: &'source str, filename: &'source Filename) -> Self {
        Self {
            string,
            filename,
            pushed_meta: Meta::default(),
            pushed_tags: TagsLinks::new(),
        }
    }

    /// Get the full str contents of the node.
    fn get_str(&self, node: Node) -> &'source str {
        &self.string[node.start_byte()..node.end_byte()]
    }

    /// Get the single char of a flag node.
    fn get_flag(&self, node: Node) -> Flag {
        Flag::try_from(self.string.as_bytes()[node.start_byte()]).unwrap_or_default()
    }

    /// Get the contents of a string-like node.
    fn get_string(&self, node: Node) -> String {
        self.string[node.start_byte() + 1..node.end_byte() - 1].replace("\\\"", "\"")
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
        s.get_string(node)
    }
}

impl FromNode for BoxStr {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "string");
        s.get_string(node).into()
    }
}

impl FromNode for CostLabel {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "string");
        s.get_string(node).into()
    }
}

impl FromNode for Account {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "account",);
        s.get_str(node).into()
    }
}

impl FromNode for Currency {
    fn from_node(node: Node, s: &ConversionState) -> Self {
        debug_assert_eq!(node.kind(), "currency",);
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
        Self::try_from_str(s.get_str(node))
            .map_err(|()| ConversionError::new(InvalidDate(s.get_str(node).into()), &node, s))
    }
}

impl TryFromNode for Booking {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "string");
        let method = s.get_string(node);
        Self::try_from(method.as_str())
            .map_err(|()| ConversionError::new(InvalidBookingMethod(method), &node, s))
    }
}

impl TryFromNode for Decimal {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        match node.kind_id() {
            node_ids::NUMBER => {
                let contents = s.get_str(node);
                Decimal::from_str_with_commas(contents).map_err(|e| {
                    ConversionError::new(InvalidDecimal(contents.into(), e.to_string()), &node, s)
                })
            }
            node_ids::PAREN_NUM_EXPR => Self::try_from_node(node.required_child(1), s),
            node_ids::UNARY_NUM_EXPR => {
                let num = Self::try_from_node(node.required_child(1), s)?;
                let sign = s.get_str(node.required_child(0));
                Ok(match sign {
                    "-" => -num,
                    _ => num,
                })
            }
            node_ids::BINARY_NUM_EXPR => {
                let left = Self::try_from_node(node.required_child(0), s)?;
                let right = Self::try_from_node(node.required_child(2), s)?;
                let op = s.get_str(node.required_child(1));
                match op {
                    "+" => Ok(left + right),
                    "-" => Ok(left - right),
                    "*" => Ok(left * right),
                    _ => left
                        .checked_div(right)
                        .ok_or_else(|| ConversionError::new(DivisionFailed(left, right), &node, s)),
                }
            }
            _ => Err(ConversionError::new(
                InternalError(format!("Invalid number node: {node:?}")),
                &node,
                s,
            )),
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
            .map(|n| CostLabel::from_node(n, s));

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

        Ok(Self {
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
            .map(|n| RawAmount::try_from_node(n, s))
            .transpose()?
            .unwrap_or_default();
        let price_annotation = node.child_by_field_id(node_fields::PRICE_ANNOTATION);
        let price = if let Some(price_n) = price_annotation {
            if let Some(amount_n) = price_n.child(1) {
                let price_amt = RawAmount::try_from_node(amount_n, s)?;
                let total_price = price_n.kind_id() == node_ids::TOTAL_PRICE_ANNOTATION;
                Some(if total_price {
                    match (price_amt.number, units.number) {
                        (Some(price_num), Some(units_number)) => RawAmount {
                            number: Some(price_num.checked_div(units_number.abs()).ok_or_else(
                                || {
                                    ConversionError::new(
                                        DivisionFailed(price_num, units_number),
                                        &node,
                                        s,
                                    )
                                },
                            )?),
                            ..price_amt
                        },
                        _ => price_amt,
                    }
                } else {
                    price_amt
                })
            } else {
                Some(RawAmount::default())
            }
        } else {
            None
        };
        Ok(Self {
            meta: EntryMeta::new(
                node.child_by_field_id(node_fields::METADATA)
                    .map(|n| Meta::try_from_node(n, s))
                    .transpose()?
                    .unwrap_or_default(),
                s.filename.clone(),
                node.line_number(),
            ),
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

impl TryFromNode for RawAmount {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert!(node.kind() == "amount" || node.kind() == "incomplete_amount",);
        Ok(Self::new(
            node.child_by_field_id(node_fields::NUMBER)
                .map(|n| Decimal::try_from_node(n, s))
                .transpose()?,
            node.child_by_field_id(node_fields::CURRENCY)
                .map(|n| Currency::from_node(n, s)),
        ))
    }
}

impl TryFromNode for Amount {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert!(node.kind() == "amount" || node.kind() == "amount_with_tolerance",);
        Ok(Self::new(
            Decimal::try_from_node(node.required_child_by_id(node_fields::NUMBER), s)?,
            Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
        ))
    }
}

impl TryFromNode for MetaValue {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        Ok(match node.kind_id() {
            node_ids::STRING => Self::String(String::from_node(node, s)),
            node_ids::DATE => Self::Date(Date::try_from_node(node, s)?),
            node_ids::TAG => Self::Tag(s.get_tag_link(node).into()),
            node_ids::ACCOUNT => Self::Account(Account::from_node(node, s)),
            node_ids::BOOL => Self::Bool(s.get_str(node) == "TRUE"),
            node_ids::AMOUNT => Self::Amount(Amount::try_from_node(node, s)?),
            node_ids::CURRENCY => Self::Currency(Currency::from_node(node, s)),
            _ => Self::Decimal(Decimal::try_from_node(node, s)?),
        })
    }
}

impl TryFromNode for MetaKeyValuePair {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "key_value");
        Ok(Self::new(
            s.get_key(node.required_child(0)).into(),
            if let Some(n) = node.child(1) {
                Some(MetaValue::try_from_node(n, s)?)
            } else {
                None
            },
        ))
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

/// Helper struct for parsing entry common fields (date, tags, links, meta).
struct ParsedEntryCommon {
    date: Date,
    tags: TagsLinks,
    links: TagsLinks,
    meta: EntryMeta,
}

impl TryFromNode for ParsedEntryCommon {
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
        }
        Ok(Self {
            date: Date::try_from_node(node.required_child_by_id(node_fields::DATE), s)?,
            tags,
            links,
            meta: EntryMeta::new(
                node.child_by_field_id(node_fields::METADATA)
                    .map(|n| Meta::try_from_node(n, s))
                    .transpose()?
                    .unwrap_or_default(),
                s.filename.clone(),
                node.line_number(),
            ),
        })
    }
}

impl TryFromNode for Balance {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "balance");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        let amt = node.required_child_by_id(node_fields::AMOUNT);
        let tol = amt.child_by_field_id(node_fields::TOLERANCE);
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            amount: Amount::try_from_node(amt, s)?,
            tolerance: tol.map(|n| Decimal::try_from_node(n, s)).transpose()?,
        })
    }
}

impl TryFromNode for Commodity {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "commodity");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            currency: Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
        })
    }
}

impl TryFromNode for Close {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
        })
    }
}

impl TryFromNode for Custom {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "custom");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            r#type: String::from_node(node.required_child_by_id(node_fields::NAME), s),
            values: node
                .children(&mut node.walk())
                .skip(3)
                .map(|n| MetaValue::try_from_node(n, s).map(CustomValue))
                .collect::<ConversionResult<_>>()?,
        })
    }
}

impl TryFromNode for Document {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "document");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        let raw_path = String::from_node(node.required_child_by_id(node_fields::FILENAME), s);
        let filename = AbsoluteUTF8Path::from_path_maybe_relative(&raw_path, s.filename)
            .map_err(|e| ConversionError::new(InvalidDocumentFilename(e.to_string()), &node, s))?;

        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            filename,
        })
    }
}

impl TryFromNode for Event {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "event");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            r#type: String::from_node(node.required_child_by_id(node_fields::TYPE), s),
            description: String::from_node(node.required_child_by_id(node_fields::DESCRIPTION), s),
        })
    }
}

impl TryFromNode for Note {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "note");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            account: Account::from_node(node.required_child_by_id(node_fields::ACCOUNT), s),
            comment: String::from_node(node.required_child_by_id(node_fields::NOTE), s),
        })
    }
}

impl TryFromNode for Open {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
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
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
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
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            currency: Currency::from_node(node.required_child_by_id(node_fields::CURRENCY), s),
            amount: Amount::try_from_node(node.required_child_by_id(node_fields::AMOUNT), s)?,
        })
    }
}

impl TryFromNode for RawTransaction {
    fn try_from_node(node: Node, s: &ConversionState) -> ConversionResult<Self> {
        debug_assert_eq!(node.kind(), "transaction");
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            flag: s.get_flag(node.required_child_by_id(node_fields::FLAG)),
            payee: node
                .child_by_field_id(node_fields::PAYEE)
                .map(|n| BoxStr::from_node(n, s)),
            narration: node
                .child_by_field_id(node_fields::NARRATION)
                .map(|n| BoxStr::from_node(n, s))
                .unwrap_or_default(),
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
        let common = ParsedEntryCommon::try_from_node(node, s)?;
        Ok(Self {
            date: common.date,
            tags: common.tags,
            links: common.links,
            meta: common.meta,
            name: String::from_node(node.required_child_by_id(node_fields::NAME), s),
            query_string: String::from_node(node.required_child_by_id(node_fields::QUERY), s),
        })
    }
}
