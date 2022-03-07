/// These are the numerical IDs for the nodes in the tree-sitter grammar.

// They should match the corresponding enum in the grammars parser.c and are
// automatically updated by the build script.
pub const ACCOUNT: u16 = 48;
pub const AMOUNT: u16 = 86;
pub const BALANCE: u16 = 60;
pub const BINARY_NUM_EXPR: u16 = 91;
pub const BOOL: u16 = 40;
pub const CLOSE: u16 = 61;
pub const COMMODITY: u16 = 62;
pub const CUSTOM: u16 = 63;
pub const DATE: u16 = 41;
pub const DOCUMENT: u16 = 64;
pub const EVENT: u16 = 65;
pub const INCLUDE: u16 = 52;
pub const NOTE: u16 = 66;
pub const NUMBER: u16 = 47;
pub const OPEN: u16 = 67;
pub const OPTION: u16 = 53;
pub const PAD: u16 = 68;
pub const PAREN_NUM_EXPR: u16 = 89;
pub const PLUGIN: u16 = 54;
pub const POPMETA: u16 = 58;
pub const POPTAG: u16 = 56;
pub const PRICE: u16 = 69;
pub const PUSHMETA: u16 = 57;
pub const PUSHTAG: u16 = 55;
pub const QUERY: u16 = 71;
pub const STRING: u16 = 45;
pub const TAG: u16 = 43;
pub const TRANSACTION: u16 = 70;
pub const TOTAL_COST: u16 = 73;
pub const TOTAL_PRICE_ANNOTATION: u16 = 79;
pub const UNARY_NUM_EXPR: u16 = 90;

#[cfg(test)]
mod tests {

    use tree_sitter::Language;

    use super::*;

    fn get_id(l: Language, name: &str) -> u16 {
        l.id_for_node_kind(name, true)
    }

    #[test]
    fn it_has_correct_ids() {
        let l = crate::parse::get_beancount_language();
        assert_eq!(get_id(l, "balance"), BALANCE);
        assert_eq!(get_id(l, "close"), CLOSE);
        assert_eq!(get_id(l, "commodity"), COMMODITY);
        assert_eq!(get_id(l, "custom"), CUSTOM);
        assert_eq!(get_id(l, "document"), DOCUMENT);
        assert_eq!(get_id(l, "event"), EVENT);
        assert_eq!(get_id(l, "note"), NOTE);
        assert_eq!(get_id(l, "open"), OPEN);
        assert_eq!(get_id(l, "pad"), PAD);
        assert_eq!(get_id(l, "price"), PRICE);
        assert_eq!(get_id(l, "transaction"), TRANSACTION);
        assert_eq!(get_id(l, "query"), QUERY);
    }
}
