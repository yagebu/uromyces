use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{path::Path, str::FromStr};

use regex::Regex;

use crate::types::{Amount, Currency, Decimal, Filename, RawEntry, RawPosting};

/// Detect CI
fn is_ci() -> bool {
    std::env::var("CI").is_ok()
}

/// Test helper to create a Currence from a string like `EUR`
pub fn c(cur: &str) -> Currency {
    cur.into()
}

/// Test helper to create a Decimal from a string like `4.00`
pub fn d(dec: &str) -> Decimal {
    Decimal::from_str_exact(dec).unwrap()
}

/// Test helper to create an Amount from a string like `4.00 USD`
pub fn a(amt: &str) -> Amount {
    Amount::from_str(amt).unwrap()
}

/// Create postings from a slice of string slices.
pub fn postings_from_strings(postings: &[&str]) -> Vec<RawPosting> {
    let string = "2000-01-01 *\n ".to_owned() + &postings.join("\n ") + "\n";
    let mut res = crate::parse::parse_string(&string, &Filename::new_dummy("string"));
    assert_eq!(res.entries.len(), 1);
    let entry = res.entries.pop().unwrap();
    match entry {
        RawEntry::Transaction(t) => t.postings,
        _ => panic!("expected transaction"),
    }
}

/// Work with snapshot tests from Beancount files.
///
/// This expects the test to come from a file which has a header, followed by some input lines and
/// then lines with the snapshot output. Both the header and the snapshot consist of lines only
/// starting with `;` to mark them as comments in the Beancount file.
///
/// The header separator line consists of 79 characters, a `;` followed by `=`s (an input with at
/// least 10 is understood). After this first line, there should be a line with a title and then
/// another header separator line. Then follows the input and after another separator line (`;`
/// followed by `-` this time), the (commented-out) snapshot result is printed.
///
/// Example of a snapshot file:
///
/// ```
/// ;==========================================================
/// ; TITLE
/// ;==========================================================
///
/// 2000-12-12 open Assets:Account
///
/// ;----------------------------------------------------------
/// ; value=expected
/// ; another_value=expected
/// ```
pub struct BeancountSnapshot {
    path: Option<PathBuf>,
    title: String,
    contents: String,
    input: String,
    snapshot: String,
    new_snapshot: String,
}

impl BeancountSnapshot {
    /// Load from a path.
    pub fn load(path: &Path) -> Self {
        let input = fs::read_to_string(path).unwrap();
        let mut res = Self::from_string(input);
        res.path = Some(path.to_owned());
        res
    }

    /// Get a reference to the title.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Get a reference to the input string.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Start a new output group - adds a delimiting line if there is output already.
    pub fn start_group(&mut self) {
        if !self.new_snapshot.is_empty() {
            self.new_snapshot += &"-".repeat(77);
            self.new_snapshot += "\n";
        }
    }

    /// Add output to snaphot, printing with Debug.
    ///
    /// This can be called multiple times to append more output.
    pub fn add_debug_output(&mut self, name: &str, value: impl std::fmt::Debug) {
        self.add_output(&format!("{name}={value:#?}\n"));
    }

    /// Add output to snaphot.
    ///
    /// This can be called multiple times to append more output.
    pub fn add_output(&mut self, output: &str) {
        self.new_snapshot += output;
    }

    /// Write the updated snapshot.
    pub fn write(&self) {
        let path = self.path.as_ref().expect("snapshot to have a path");
        let new_contents = self.print_to_string();
        if new_contents != self.contents {
            if is_ci() {
                assert_eq!(new_contents, self.contents, "snapshot failed");
            } else {
                fs::write(path, self.print_to_string()).expect("write to work in test");
            }
        }
    }

    /// Print out a snapshot in the defined format.
    fn print_to_string(&self) -> String {
        format!(
            r"{comment:=<79}
; {title}
{comment:=<79}
{input}{comment:-<79}
; {output}
",
            comment = ";",
            title = self.title,
            input = self.input,
            output = self.new_snapshot.lines().collect::<Vec<_>>().join("\n; "),
        )
    }

    /// Load from string.
    fn from_string(contents: String) -> BeancountSnapshot {
        static SNAPSHOT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^(?x)
              ;={3,}\n
              ;\ (?<title>.*?)\n
              ;={3,}\n
              (?<input>(.|\n)*?)
              (
                  ;-{3,}\n
                  (?<snapshot>(;\ .*\n)+)
              )?
              $",
            )
            .expect("static regex to compile")
        });
        let capture = SNAPSHOT_REGEX
            .captures(&contents)
            .expect("snapshot_regex to match provided input");
        let snapshot = capture
            .name("snapshot")
            .map_or("", |m| m.as_str())
            .split("; ")
            .collect::<String>();
        let current_snapshot_len = snapshot.len();
        let title = capture["title"].to_string();
        let input = capture["input"].to_string();

        BeancountSnapshot {
            contents,
            path: None,
            title,
            input,
            snapshot,
            new_snapshot: String::with_capacity(current_snapshot_len),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_snapshot_without_output() {
        let contents = r";========================
; TITLE
;===============
INPUT

LINES
";
        let snapshot = BeancountSnapshot::from_string(contents.to_string());
        assert_eq!(snapshot.title, "TITLE");
        assert_eq!(snapshot.input, "INPUT\n\nLINES\n");
        assert_eq!(snapshot.snapshot, "");
    }

    #[test]
    fn test_match_snapshot_with_output_roundtrip() {
        let contents = r";==============================================================================
; TITLE
;==============================================================================
INPUT

LINES
;------------------------------------------------------------------------------
; snapshot_line1
; snapshot_line2
";
        let mut snapshot = BeancountSnapshot::from_string(contents.to_string());
        assert_eq!(snapshot.title, "TITLE");
        assert_eq!(snapshot.input, "INPUT\n\nLINES\n");
        assert_eq!(snapshot.snapshot, "snapshot_line1\nsnapshot_line2\n");

        snapshot.new_snapshot = "snapshot_line1\nsnapshot_line2\n".to_string();
        assert_eq!(&snapshot.print_to_string(), contents);
    }

    #[test]
    fn test_c() {
        assert_eq!(c("EUR"), Currency::from("EUR"));
    }

    #[test]
    fn test_d() {
        assert_eq!(d("1"), Decimal::ONE);
    }

    #[test]
    fn test_a() {
        assert_eq!(a("1 EUR"), Amount::new(Decimal::ONE, c("EUR")));
    }

    #[test]
    #[should_panic(expected = "called `Result::unwrap()")]
    fn test_a_panic() {
        a("10");
    }
}
