//! Finding documents.

use crate::errors::UroError;
use crate::ledgers::Ledger;
use crate::types::{Account, Date, Document, Entry, EntryHeader};

/// Get a sorted list of all open accounts in the ledger.
fn get_all_open_accounts(ledger: &Ledger) -> Vec<&Account> {
    let mut all_accounts = ledger
        .entries
        .iter()
        .filter_map(|e| {
            if let Entry::Open(o) = e {
                Some(&o.account)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    // Sort so that we find the documents in a consistent order, which is also independent of the
    // order of open directives.
    all_accounts.sort_unstable();
    // dedup just in case
    all_accounts.dedup();
    all_accounts
}

/// Find documents for the specified document options of the ledger.
pub fn find(ledger: &Ledger) -> (Vec<Entry>, Vec<UroError>) {
    let document_paths = &ledger.options.documents;
    if document_paths.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut new_documents = Vec::new();
    let mut new_errors = Vec::new();

    let all_accounts = get_all_open_accounts(ledger);

    for document_path in document_paths {
        let documents_dir = ledger.filename.join_relative_to_file(document_path);
        if !documents_dir.as_ref().is_dir() {
            new_errors.push(
                UroError::new(format!(
                    "Could not read documents directory: '{documents_dir}'"
                ))
                .with_filename(&ledger.filename),
            );
            continue;
        }

        for account in &all_accounts {
            let account_dir = documents_dir.join_account(account);
            if !account_dir.as_ref().is_dir() {
                // Ignore missing directories and the like.
                continue;
            }

            let Ok(read_dir) = account_dir.as_ref().read_dir() else {
                // The directory exists (checked above), but there seems to be some other problem
                // reading from it, so surface an error.
                new_errors.push(
                    UroError::new(format!(
                        "Could not read documents directory: '{account_dir}'"
                    ))
                    .with_filename(&ledger.filename),
                );
                continue;
            };

            let mut account_files = read_dir
                // only consider DirEntries that we were read without error
                .filter_map(std::result::Result::ok)
                // only consider files
                .filter_map(|dir_entry| {
                    if dir_entry.file_type().ok()?.is_file() {
                        Some(dir_entry)
                    } else {
                        None
                    }
                })
                // Only consider Unicode filenames
                .filter_map(|dir_entry| Some(dir_entry.file_name().to_str()?.to_string()))
                .collect::<Vec<_>>();
            account_files.sort_unstable();

            new_documents.extend(&mut account_files.iter().filter_map(|file_name| {
                if let Ok(date) = Date::try_from_str(file_name) {
                    Some(Document {
                        header: EntryHeader::new(date, Some(ledger.filename.clone()), 0),
                        account: (*account).clone(),
                        filename: account_dir.join(file_name),
                    })
                } else {
                    None
                }
            }));
        }
    }

    (
        new_documents.into_iter().map(Entry::Document).collect(),
        new_errors,
    )
}
