use std::path::Path;

use crate::errors::UroError;
use crate::test_utils::BeancountSnapshot;

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

    if !parsed.directives.is_empty() {
        snapshot.add_debug_output("directives", parsed.directives);
    }

    snapshot.write();
}

#[test]
fn parser_snapshot_tests() {
    insta::glob!("bean_snaps_parser/*.beancount", |path| {
        run_parser_snapshot_test(path);
    });
}
