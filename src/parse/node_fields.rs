/// These are the numerical IDs for the fields in the tree-sitter grammar.

// They should match the corresponding enum in the grammars parser.c and are
// automatically updated by the build script.
pub const ACCOUNT: u16 = 1;
pub const AMOUNT: u16 = 2;
pub const BOOKING: u16 = 3;
pub const COMPOUND_AMOUNT: u16 = 4;
pub const COST_SPEC: u16 = 7;
pub const CURRENCIES: u16 = 8;
pub const CURRENCY: u16 = 9;
pub const DATE: u16 = 10;
pub const DESCRIPTION: u16 = 11;
pub const FILENAME: u16 = 12;
pub const FLAG: u16 = 13;
pub const FROM_ACCOUNT: u16 = 14;
pub const MERGE: u16 = 17;
pub const METADATA: u16 = 18;
pub const NAME: u16 = 19;
pub const NARRATION: u16 = 20;
pub const NOTE: u16 = 21;
pub const NUMBER: u16 = 22;
pub const NUMBER_PER: u16 = 23;
pub const NUMBER_TOTAL: u16 = 24;
pub const PAYEE: u16 = 25;
pub const POSTINGS: u16 = 26;
pub const PRICE_ANNOTATION: u16 = 27;
pub const QUERY: u16 = 28;
pub const STRING: u16 = 29;
pub const TAGS_AND_LINKS: u16 = 31;
pub const TOLERANCE: u16 = 32;
pub const TYPE: u16 = 33;

#[cfg(test)]
mod tests {

    use tree_sitter::Language;

    use super::*;

    fn get_id(l: Language, name: &str) -> u16 {
        l.field_id_for_name(name).expect("")
    }

    #[test]
    fn it_has_correct_ids() {
        let l = crate::parse::get_beancount_language();
        assert_eq!(get_id(l, "account"), ACCOUNT);
        assert_eq!(get_id(l, "amount"), AMOUNT);
        assert_eq!(get_id(l, "booking"), BOOKING);
        assert_eq!(get_id(l, "currencies"), CURRENCIES);
        assert_eq!(get_id(l, "currency"), CURRENCY);
        assert_eq!(get_id(l, "description"), DESCRIPTION);
        assert_eq!(get_id(l, "filename"), FILENAME);
        assert_eq!(get_id(l, "flag"), FLAG);
        assert_eq!(get_id(l, "from_account"), FROM_ACCOUNT);
        assert_eq!(get_id(l, "name"), NAME);
        assert_eq!(get_id(l, "narration"), NARRATION);
        assert_eq!(get_id(l, "note"), NOTE);
        assert_eq!(get_id(l, "number"), NUMBER);
        assert_eq!(get_id(l, "payee"), PAYEE);
        assert_eq!(get_id(l, "postings"), POSTINGS);
        assert_eq!(get_id(l, "price_annotation"), PRICE_ANNOTATION);
        assert_eq!(get_id(l, "query"), QUERY);
        assert_eq!(get_id(l, "tolerance"), TOLERANCE);
        assert_eq!(get_id(l, "type"), TYPE);
    }
}
