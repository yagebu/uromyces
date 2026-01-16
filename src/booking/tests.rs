use std::path::Path;

use crate::errors::UroError;
use crate::inventory::Position;
use crate::ledgers::RawLedger;
use crate::parse::parse_string;
use crate::test_utils;
use crate::types::{
    AbsoluteUTF8Path, Account, Booking, Entry, MIN_DATE, Posting, RawEntry, RawPosting,
    RawTransaction,
};

use super::book_entries;
use super::complete_cost_spec;

const APPLY: &str = "apply";
const ANTE: &str = "ante";
const BOOKED: &str = "booked";

/// Find the first transaction for the given tag.
fn find_first_with_tag(tag: &str, txns: &[RawEntry]) -> Option<RawTransaction> {
    txns.iter()
        .filter_map(|e| e.as_transaction())
        .find(|t| t.tags.contains(tag))
        .cloned()
}

/// Compare postings
///
/// - same number of postings
/// - for each posting:
///     - same units
///     - same cost
fn compare_postings(expected: &[RawPosting], booked: &[Posting]) {
    assert_eq!(expected.len(), booked.len());

    for (posting_expected, posting_booked) in std::iter::zip(expected, booked) {
        assert_eq!(
            posting_expected.units.to_string(),
            posting_booked.units.to_string()
        );
        if let Some(expected_cost) = &posting_expected.cost {
            let expected = complete_cost_spec(expected_cost, MIN_DATE, None).unwrap();
            assert_eq!(Some(expected), posting_booked.cost);
        } else {
            assert!(posting_booked.cost.is_none());
        }
    }
}

fn run_booking_test(path: &Path) {
    let mut snapshot = test_utils::BeancountSnapshot::load(path);

    let filename: AbsoluteUTF8Path = path.try_into().unwrap();
    let raw_ledger = RawLedger::from_single_parsed_file(
        filename.clone().into(),
        parse_string(snapshot.input(), &filename.clone().into()),
    );
    let title = snapshot.title();
    let booking_methods = [
        "STRICT_WITH_SIZE",
        "AVERAGE",
        "STRICT",
        "FIFO",
        "HIFO",
        "LIFO",
        "NONE",
    ];
    let booking_method: Booking = (*booking_methods
        .iter()
        .find(|m| title.starts_with(*m))
        .expect("valid booking method prefix"))
    .try_into()
    .expect("valid booking method");
    let entries = &raw_ledger.entries;
    let txns_apply = entries
        .iter()
        .filter_map(|e| e.as_transaction())
        .filter(|t| t.tags.contains(APPLY))
        .cloned()
        .collect::<Vec<_>>();

    // assert that the input parsed correctly and we have at least one #apply transaction
    assert!(raw_ledger.errors.is_empty());
    assert!(!txns_apply.is_empty());

    for apply_txn in txns_apply {
        snapshot.start_group();

        let mut ledger = raw_ledger.clone();
        ledger.options.booking_method = booking_method;
        ledger.entries = vec![];

        if let Some(ante_txn) = find_first_with_tag(ANTE, entries) {
            ledger.entries.push(ante_txn.into());
        }
        ledger.entries.push(apply_txn.into());

        let (booked, balances) = book_entries(ledger.clone());

        snapshot.add_debug_output(
            "errors",
            booked
                .errors
                .iter()
                .map(UroError::message)
                .collect::<Vec<_>>(),
        );

        let Some(last_entry) = booked.entries.last() else {
            continue;
        };
        let Entry::Transaction(last_txn) = last_entry else {
            continue;
        };

        if booked.errors.is_empty() {
            snapshot.add_debug_output(
                "booked",
                last_txn
                    .postings
                    .iter()
                    .map(Position::print_units_and_cost)
                    .collect::<Vec<_>>(),
            );
        }

        let account: Account = "Assets:Account".into();
        let balance = balances.get(&account).cloned().unwrap_or_default();
        snapshot.add_debug_output(
            "ex_balances",
            balance
                .iter()
                .map(|p| p.print_units_and_cost())
                .collect::<Vec<_>>(),
        );

        if let Some(expected_booked_txn) = find_first_with_tag(BOOKED, entries) {
            if expected_booked_txn.meta.contains_key("error") {
                assert!(!booked.errors.is_empty());
            } else {
                assert!(booked.errors.is_empty());
                // compare the expected (booked_txn) to the result (last txn in booked).
                if !last_txn.postings.is_empty() {
                    compare_postings(&expected_booked_txn.postings, &last_txn.postings);
                }
            }
        }
    }

    snapshot.write();
}

/// This test is based on DSL for booking tests in Beancount in `beancount.parser.booking_full_test`.
///
/// The Python test uses mocks and allows assertions (with the `reduced`, `ambi-matches`,
/// `ambi-resolved` tags. Those are ignored in our implementation here. However, comparing the
/// snapshot outputs allows for similar validations.
///
/// The test inputs can be imported from Beancount with `contrib/scripts.py`.
#[test]
fn booking_test() {
    insta::glob!("booking_tests/*.beancount", |path| {
        run_booking_test(path);
    });
    insta::glob!("booking_full_tests_imported/*.beancount", |path| {
        run_booking_test(path);
    });
}
