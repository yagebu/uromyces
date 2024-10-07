use std::str::FromStr;

use crate::types::{Amount, Currency, Decimal, RawEntry, RawPosting};

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
    let mut res = crate::parse::parse_string(&string, &None);
    assert_eq!(res.entries.len(), 1);
    let entry = res.entries.pop().unwrap();
    match entry {
        RawEntry::Transaction(t) => t.postings,
        _ => panic!("expected transaction"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
