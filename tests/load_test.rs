use uromyces::types::AbsoluteUTF8Path;

fn snap_ledger(snap_name: &str, filename: &str) {
    let current_dir = std::env::current_dir().expect("test to obtain its working dir");
    let settings = {
        let mut settings = insta::Settings::clone_current();
        let cwd = current_dir
            .to_str()
            .expect("this test to run in a Unicode path");
        // Escape the path for use as a regex pattern (handles Windows backslashes)
        let cwd_escaped = regex::escape(cwd);
        settings.add_filter(&cwd_escaped, "[REPO_DIR]");
        settings.remove_input_file();
        settings
    };
    let path = current_dir.join("tests").join("ledgers").join(filename);
    let file_path: AbsoluteUTF8Path = path
        .as_path()
        .try_into()
        .expect("FilePath creation to work");
    let mut ledger = uromyces::load(file_path);
    ledger.run_validations();
    settings.bind(|| {
        insta::assert_json_snapshot!(snap_name, ledger);
    });
}

// Since the snapshots contain paths in the JSON outputs and converting from / to windows-style
// paths would be complicated, we do not run them on Windows right now
#[test]
#[cfg(not(target_os = "windows"))]
fn test_ledger_snapshots() {
    snap_ledger("loads_example_file", "example.beancount");
    snap_ledger("errors_on_invalid_ledger", "invalid-input.beancount");
    snap_ledger("missing_file", "non-existent-file-missing.beancount");
    snap_ledger("loads_short_example_file", "short-example.beancount");
    snap_ledger("handles_includes", "test-includes.beancount");
    snap_ledger("reads_document_dir", "documents.beancount");
    snap_ledger("pad_entries", "pad.beancount");
}
