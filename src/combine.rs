//! Load files and combine multiple parse results into one (raw) ledger.

use std::collections::VecDeque;
use std::fs;

use hashbrown::HashSet;

use crate::booking;
use crate::display_precision::DisplayPrecisionsStats;
use crate::errors::UroError;
use crate::ledgers::{Ledger, RawLedger};
use crate::parse;
use crate::parse::ParsedFile;
use crate::types::{AbsoluteUTF8Path, Filename, Plugin, RawDirective};
use crate::util::paths;
use crate::util::timer::SimpleTimer;

struct PathAndResult {
    path: Filename,
    result: ParsedFile,
}

/// Load a Beancount file.
///
/// Takes a path and tries to load the given Beancount file, producing a completely
/// booked list of entries.
///
/// This does not run any user-specified plugins or the built-in validations, those
/// should be orchestrated from the calling Python code.
#[must_use]
pub fn load(main_path: AbsoluteUTF8Path) -> Ledger {
    let paths_and_results = load_beancount_file(main_path);
    let raw_ledger = combine_files(paths_and_results);

    let mut t = SimpleTimer::new();
    let (mut ledger, _) = booking::book_entries(raw_ledger);
    t.log_elapsed("booking");

    crate::plugins::run_pre(&mut ledger);
    ledger
}

/// Load a Beancount string.
///
/// Takes a string and tries parse it as a Beancount file, producing a completely
/// booked list of entries.
///
/// This does not run any user-specified plugins or the built-in validations, those
/// should be orchestrated from the calling Python code.
#[must_use]
pub fn load_string(string: &str, filename: Filename) -> Ledger {
    let result = parse::parse_string(string, &filename);
    let paths_and_results = vec![PathAndResult {
        result,
        path: filename,
    }];
    let raw_ledger = combine_files(paths_and_results);

    let mut t = SimpleTimer::new();
    let (mut ledger, _) = booking::book_entries(raw_ledger);
    t.log_elapsed("booking");

    crate::plugins::run_pre(&mut ledger);
    ledger
}

/// Load and parse a single Beancount file.
fn load_single_beancount_file(path: &AbsoluteUTF8Path) -> Result<ParsedFile, UroError> {
    // Always append a newline at the end, to avoid errors on a last missing end-of-line.
    let string = fs::read_to_string(path).map_err(|io_error| {
        UroError::new(format!("Could not read file due to IO error: {io_error}"))
            .with_filename(path.clone().into())
    })?;
    let mut t = SimpleTimer::new();
    let result = parse::parse_string(&string, &path.clone().into());
    t.log_elapsed(&format!("{path}: parsing"));
    Ok(result)
}

/// Load and parse a Beancount file and all includes.
fn load_beancount_file(main_path: AbsoluteUTF8Path) -> Vec<PathAndResult> {
    let mut path_queue = VecDeque::new();
    path_queue.push_back(main_path);
    // keep track of loaded files to avoid doing them twice
    let mut loaded = HashSet::new();
    let mut results = Vec::new();

    while let Some(path) = path_queue.pop_front() {
        // Check if that we have not seen this file.
        if loaded.insert(path.clone()) {
            let mut result = match load_single_beancount_file(&path) {
                Ok(res) => res,
                Err(err) => ParsedFile::from_error(err),
            };
            for directive in &result.directives {
                if let RawDirective::Include { pattern } = directive {
                    match paths::glob_include(&path, pattern) {
                        Ok(included_paths) => path_queue.extend(included_paths.into_iter()),
                        Err(glob_include_error) => result.errors.push(
                            UroError::new(format!(
                                "Include pattern '{pattern}' failed: {glob_include_error}"
                            ))
                            .with_filename(path.clone().into()),
                        ),
                    }
                }
            }
            results.push(PathAndResult {
                path: path.into(),
                result,
            });
        }
    }
    results
}

/// Combine the parsed results from multiple files.
///
/// With all files at hand, we can:
/// - Get the complete options for this ledger.
/// - Combine raw entries and options into one Vec each
fn combine_files(result: Vec<PathAndResult>) -> RawLedger {
    let all_includes = result.iter().map(|r| r.path.clone()).collect::<Vec<_>>();
    let entry_count = result.iter().map(|r| r.result.entries.len()).sum();
    let mut combined =
        RawLedger::from_filename_and_includes(result[0].path.clone(), all_includes, entry_count);
    let mut t = SimpleTimer::new();

    // Merge all ledgers
    for PathAndResult {
        path: _,
        mut result,
    } in result
    {
        combined
            .options
            .update_from_raw_directives(&result.directives);
        combined.entries.append(&mut result.entries);
        combined.errors.append(&mut result.errors);
        combined.plugins.append(
            &mut result
                .directives
                .into_iter()
                .filter_map(|d| {
                    if let RawDirective::Plugin { name, config } = d {
                        Some(Plugin { name, config })
                    } else {
                        None
                    }
                })
                .collect(),
        );
    }
    t.log_elapsed("combining options and entries");

    combined.entries.sort();
    t.log_elapsed("sorting entries");

    combined.options.display_precisions =
        DisplayPrecisionsStats::from_raw_entries(&combined.entries).get_precisions();
    t.log_elapsed("compute display context");

    combined
}
