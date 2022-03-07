use super::errors::{BookingError, BookingErrorKind};
use crate::types::{CostSpec, Currency, IncompleteAmount, RawPosting};

/// Get the currency group for this posting.
fn get_posting_currency_group(p: &RawPosting) -> Option<Currency> {
    match (&p.cost, &p.price) {
        // If there is a cost currency, use it
        (
            Some(CostSpec {
                currency: Some(cost_currency),
                ..
            }),
            _,
        ) => Some(cost_currency.clone()),
        // If there is a price currency, use that next
        (
            _,
            Some(IncompleteAmount {
                currency: Some(price_currency),
                ..
            }),
        ) => Some(price_currency.clone()),
        // If both price and cost are missing, use the units currency
        (None, None) => p.units.currency.clone(),
        // Otherwise, it's None
        _ => None,
    }
}

type VecMap<K, V> = Vec<(K, Vec<V>)>;

type GroupedPostings = VecMap<Currency, RawPosting>;

/// Push an item to the matching group (abusing the Vec as a kind of Map).
fn push_to_group<K, V>(groups: &mut VecMap<K, V>, key: &K, value: V)
where
    K: PartialEq + Clone,
{
    let group = groups.iter_mut().find(|group| group.0 == *key);
    if let Some((_, ps)) = group {
        ps.push(value);
    } else {
        groups.push((key.clone(), vec![value]));
    }
}

/// Group by and fill in missing currencies in the given postings.
///
/// We go through all postings:
///
/// - if one of price and cost currency is set and the other is missing, we fill in the missing value.
/// - we group postings by the currency that their weight is in (basically cost, price, units in that order)
/// - one posting is allowed to be an "auto posting" without units, of which we create one copy for each
///   currency group.
///
/// Unlike Beancount, this does not try to fill in currencies from the account balances.
pub(super) fn group_and_fill_in_currencies(
    postings: &[RawPosting],
) -> Result<GroupedPostings, BookingError> {
    let mut auto_posting = None;
    let mut groups = Vec::new();
    let mut unknown = Vec::new();

    for mut p in postings.iter().cloned() {
        // Ensure that cost and price have the same currency.
        if let Some(cost) = &mut p.cost {
            if let Some(price) = &mut p.price {
                // Both cost and price are set, now check if one of the currencies is missing.
                // If so, set it to the other one.
                if let Some(cost_currency) = &cost.currency {
                    if price.currency.is_none() {
                        price.currency = Some(cost_currency.clone());
                    }
                }
                if let Some(price_currency) = &price.currency {
                    if cost.currency.is_none() {
                        cost.currency = Some(price_currency.clone());
                    }
                }
            }
        }

        if p.units.number.is_none() && p.units.currency.is_none() {
            if auto_posting.is_some() {
                return Err(BookingError::new(
                    &p,
                    BookingErrorKind::MultipleAutoPostings,
                ));
            }
            auto_posting = Some(p);
        } else {
            match get_posting_currency_group(&p) {
                Some(c) => push_to_group(&mut groups, &c, p),
                None => unknown.push(p),
            };
        }
    }

    // Only support this simple case for now.
    if unknown.len() < 2 && groups.len() == 1 {
        if let Some(mut unknown_posting) = unknown.pop() {
            let currency = groups[0].0.clone();
            if unknown_posting.cost.is_none() && unknown_posting.price.is_none() {
                unknown_posting.units.currency = Some(currency.clone());
            } else {
                if let Some(cost) = &mut unknown_posting.cost {
                    cost.currency = Some(currency.clone());
                }
                if let Some(price) = &mut unknown_posting.price {
                    price.currency = Some(currency.clone());
                }
            }
            push_to_group(&mut groups, &currency, unknown_posting);
        }
    }

    if let Some(auto_p) = auto_posting {
        // Add this auto posting for each currency group.
        for (currency, ps) in &mut groups {
            let mut new_posting = auto_p.clone();
            new_posting.units.currency = Some(currency.clone());
            ps.push(new_posting);
        }
    }

    // Check that no currencies are missing.
    for (_, postings) in &groups {
        for posting in postings {
            if posting.units.currency.is_none() {
                return Err(BookingError::new(
                    posting,
                    BookingErrorKind::UnresolvedUnitsCurrency,
                ));
            }
            if posting.price.iter().any(|v| v.currency.is_none()) {
                return Err(BookingError::new(
                    posting,
                    BookingErrorKind::UnresolvedPriceCurrency,
                ));
            }
            if posting.cost.iter().any(|v| v.currency.is_none()) {
                return Err(BookingError::new(
                    posting,
                    BookingErrorKind::UnresolvedCostCurrency,
                ));
            }
        }
    }

    Ok(groups)
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{c, postings_from_strings};

    use super::*;

    #[test]
    fn test_get_currency_group() {
        fn t(p: &str, e: Option<&str>) {
            let posting = &postings_from_strings(&[p])[0];
            assert_eq!(get_posting_currency_group(posting), e.map(c));
        }
        t("A:C 20 USD", Some("USD"));
        t("A:C    USD", Some("USD"));
        t("A:C       ", None);
        t("A:C 20 USD @ 20 EUR", Some("EUR"));
        t("A:C 20 USD @    EUR", Some("EUR"));
        t("A:C    USD @ 20 EUR", Some("EUR"));
        t("A:C    USD @ 20 EUR", Some("EUR"));
        t("A:C 20 USD @", None);
        t("A:C 20 USD {       }", None);
        t("A:C 20 USD {       } @", None);
        t("A:C 20 USD {10 ASDF} @ 20 EUR", Some("ASDF"));
        t("A:C 20 USD {10 ASDF} @    EUR", Some("ASDF"));
        t("A:C 20 USD {   ASDF} @ 20 EUR", Some("ASDF"));
        t("A:C 20 USD {   ASDF} @    EUR", Some("ASDF"));
        t("A:C    USD {10 ASDF} @ 20 EUR", Some("ASDF"));
        t("A:C    USD {10 ASDF} @    EUR", Some("ASDF"));
        t("A:C    USD {   ASDF} @ 20 EUR", Some("ASDF"));
        t("A:C    USD {   ASDF} @    EUR", Some("ASDF"));
    }

    fn check(ps: &[&str]) -> String {
        use std::fmt::Write;

        let posting = &postings_from_strings(ps);
        let groups = group_and_fill_in_currencies(posting).unwrap();
        let mut s = String::new();

        for (currency, postings) in groups {
            writeln!(&mut s, "{currency}").unwrap();
            for p in postings {
                writeln!(
                    &mut s,
                    "    account: {}; units: {:?}, price: {:?}, cost: {:?}",
                    p.account, p.units, p.price, p.cost
                )
                .unwrap();
            }
        }
        s
    }

    #[test]
    fn test_filling_in_complete() {
        let groups = check(&["Assets:Cash 20 USD"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
        "###);
    }

    #[test]
    fn test_filling_in_one_auto_posting() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash2"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: None, currency: Some(Currency("USD")) }, price: None, cost: None
        "###);
    }

    #[test]
    fn test_filling_in_multiple_auto_postings() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash 20 EUR", "Assets:Cash2"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: None, currency: Some(Currency("USD")) }, price: None, cost: None
        EUR
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("EUR")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: None, currency: Some(Currency("EUR")) }, price: None, cost: None
        "###);
    }

    #[test]
    fn test_filling_cost() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash2 30 APL {}"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: Some(30), currency: Some(Currency("APL")) }, price: None, cost: Some(CostSpec { number_per: None, number_total: None, currency: Some(Currency("USD")), date: None, label: None, merge: false })
        "###);
    }
    #[test]
    fn test_filling_price() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash2 30 APL @"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: Some(30), currency: Some(Currency("APL")) }, price: Some(IncompleteAmount { number: None, currency: Some(Currency("USD")) }), cost: None
        "###);
    }

    #[test]
    fn test_filling_price_and_cost() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash2 30 APL {} @"]);
        insta::assert_snapshot!(groups, @r###"
        USD
            account: Assets:Cash; units: IncompleteAmount { number: Some(20), currency: Some(Currency("USD")) }, price: None, cost: None
            account: Assets:Cash2; units: IncompleteAmount { number: Some(30), currency: Some(Currency("APL")) }, price: Some(IncompleteAmount { number: None, currency: Some(Currency("USD")) }), cost: Some(CostSpec { number_per: None, number_total: None, currency: Some(Currency("USD")), date: None, label: None, merge: false })
        "###);
    }
}
