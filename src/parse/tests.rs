use std::fs;
use std::path::Path;

use crate::errors::UroError;
use crate::test_utils::BeancountSnapshot;
use crate::types::Filename;

#[test]
fn glob_input_files() {
    let dummy_filename = Filename::new_dummy("string");
    insta::glob!("test_inputs/*.beancount", |path| {
        let input = fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(super::parse_string(&input, &dummy_filename.clone()));
    });
}

fn run_parser_snapshot_test(path: &Path) {
    let mut snapshot = BeancountSnapshot::load(path);
    let parsed = super::parse_string(snapshot.input(), &path.try_into().unwrap());

    if parsed.errors.is_empty() {
        snapshot.add_debug_output("entries", &parsed.entries);
    } else {
        snapshot.add_debug_output(
            "errors",
            parsed
                .errors
                .iter()
                .map(UroError::message)
                .collect::<Vec<_>>(),
        );
        snapshot.add_debug_output("num_entries", parsed.entries.len());
    }

    snapshot.write();
}

#[test]
fn parser_snapshot_tests() {
    insta::glob!("parser_snapshot_tests/*.beancount", |path| {
        run_parser_snapshot_test(path);
    });
}
