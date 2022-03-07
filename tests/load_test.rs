fn snap_ledger(snap_name: &str, filename: &str) {
    let mut settings = insta::Settings::clone_current();
    let current_dir = std::env::current_dir().expect("this test to obtain its working dir");
    let cwd = current_dir
        .to_str()
        .expect("this test to run in a Unicode path");
    settings.add_filter(cwd, "[REPO_DIR]");
    settings.remove_input_file();
    let path = current_dir.join("tests").join("ledgers").join(filename);
    let ledger = uromyces::load(&path.try_into().expect("FilePath creation to work"));
    settings.bind(|| {
        insta::assert_json_snapshot!(snap_name, ledger);
    });
}

#[test]
fn test_ledger_snapshots() {
    snap_ledger("loads_example_file", "example.beancount");
    snap_ledger("errors_on_invalid_ledger", "invalid-input.beancount");
    snap_ledger("missing_file", "non-existent-file-missing.beancount");
    snap_ledger("loads_short_example_file", "short-example.beancount");
    snap_ledger("handles_includes", "test-includes.beancount");
    snap_ledger("reads_document_dir", "documents.beancount");
    snap_ledger("pad_entries", "pad.beancount");
}
