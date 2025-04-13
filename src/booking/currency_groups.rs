use crate::types::{CostSpec, Currency, IncompleteAmount, RawPosting};

use super::errors::{BookingError, BookingErrorKind};
use super::AccountBalances;

/// Get the currency group for this posting.
///
/// Sees whether one of the following exists and return it, in this order:
/// - cost currency
/// - price currency
/// - units currency
/// - None otherwise
fn get_posting_currency_group(posting: &RawPosting) -> Option<&Currency> {
    match (&posting.cost, &posting.price) {
        (
            Some(CostSpec {
                currency: Some(cost_currency),
                ..
            }),
            _,
        ) => Some(cost_currency),
        (
            _,
            Some(IncompleteAmount {
                currency: Some(price_currency),
                ..
            }),
        ) => Some(price_currency),
        (None, None) => posting.units.currency.as_ref(),
        _ => None,
    }
}

type GroupedPostings = Vec<(Currency, Vec<RawPosting>)>;

/// Check whether all currencies are set in the posting.
fn check_posting_currencies(posting: &RawPosting) -> Result<(), BookingError> {
    if posting.units.currency.is_none() {
        Err(BookingErrorKind::UnresolvedUnitsCurrency.with_posting(posting))
    } else if posting.price.iter().any(|v| v.currency.is_none()) {
        Err(BookingErrorKind::UnresolvedPriceCurrency.with_posting(posting))
    } else if posting.cost.iter().any(|v| v.currency.is_none()) {
        Err(BookingErrorKind::UnresolvedCostCurrency.with_posting(posting))
    } else {
        Ok(())
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
/// Unlike Beancount, this does not try to fill in all currencies from the account balances, just
/// cost currencies, if they are missing.
pub(super) fn group_and_fill_in_currencies(
    postings: &[RawPosting],
    balances: &AccountBalances,
) -> Result<GroupedPostings, BookingError> {
    let mut auto_posting = None;
    let mut groups: GroupedPostings = Vec::new();
    let mut unknown = Vec::new();

    for mut posting in postings.iter().cloned() {
        if let (Some(cost), Some(price)) = (&mut posting.cost, &mut posting.price) {
            // Both cost and price are set, now check if one of the currencies is missing. If so,
            // set it to the other one.
            match (&cost.currency, &price.currency) {
                (Some(cost_currency), None) => {
                    price.currency = Some(cost_currency.clone());
                }
                (None, Some(price_currency)) => {
                    cost.currency = Some(price_currency.clone());
                }
                _ => {}
            }
        }

        if posting.units.number.is_none()
            && posting.units.currency.is_none()
            && posting.price.is_none()
        {
            if auto_posting.is_some() {
                return Err(BookingErrorKind::MultipleAutoPostings.with_posting(&posting));
            }
            auto_posting = Some(posting);
        } else {
            match get_posting_currency_group(&posting) {
                Some(currency) => {
                    check_posting_currencies(&posting)?;
                    if let Some((_, group_postings)) =
                        groups.iter_mut().find(|(c, _)| c == currency)
                    {
                        group_postings.push(posting);
                    } else {
                        groups.push((currency.clone(), vec![posting]));
                    }
                }
                None => unknown.push(posting),
            }
        }
    }

    // Only support this simple case for now.
    if unknown.len() < 2 && groups.len() == 1 {
        if let Some(mut unknown_posting) = unknown.pop() {
            let currency = &groups[0].0;
            match (&mut unknown_posting.cost, &mut unknown_posting.price) {
                (None, None) => {
                    unknown_posting.units.currency = Some(currency.clone());
                }
                (Some(cost), None) => {
                    cost.currency = Some(currency.clone());
                }
                (None, Some(price)) => {
                    price.currency = Some(currency.clone());
                }
                (Some(cost), Some(price)) => {
                    cost.currency = Some(currency.clone());
                    price.currency = Some(currency.clone());
                }
            }
            check_posting_currencies(&unknown_posting)?;
            groups[0].1.push(unknown_posting);
        }
    }

    // If we had more than one unknown posting, we infer cost currencies from existing account
    // balances.
    // Otherwise, we will bubble up an error.
    for mut posting in unknown {
        if let Some(balance) = balances.get(&posting.account) {
            if let Some(ref mut cost) = posting.cost {
                if cost.currency.is_none() {
                    let cost_currencies = balance.cost_currencies();
                    if cost_currencies.len() == 1 {
                        cost.currency = cost_currencies.into_iter().next().cloned();
                    }
                }
            }
        }
        check_posting_currencies(&posting)?;
        let currency =
            get_posting_currency_group(&posting).expect("we just checked it has currencies");
        if let Some((_, group_postings)) = groups.iter_mut().find(|(c, _)| c == currency) {
            group_postings.push(posting);
        } else {
            groups.push((currency.clone(), vec![posting]));
        }
    }

    if let Some(auto_posting) = auto_posting {
        // Add this auto posting for each currency group.
        for (currency, group_postings) in &mut groups {
            let mut new_posting = auto_posting.clone();
            new_posting.units.currency = Some(currency.clone());
            check_posting_currencies(&new_posting)?;
            group_postings.push(new_posting);
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
            assert_eq!(get_posting_currency_group(posting), e.map(c).as_ref());
        }
        t("Assets:Cash     20 USD", Some("USD"));
        t("Assets:Cash        USD", Some("USD"));
        t("Assets:Cash           ", None);
        t("Assets:Cash     20 USD @ 20 EUR", Some("EUR"));
        t("Assets:Cash     20 USD @    EUR", Some("EUR"));
        t("Assets:Cash        USD @ 20 EUR", Some("EUR"));
        t("Assets:Cash        USD @ 20 EUR", Some("EUR"));
        t("Assets:Cash     20 USD @", None);
        t("Assets:Cash     20 USD {       }", None);
        t("Assets:Cash     20 USD {       } @", None);
        t("Assets:Cash     20 USD {10 ASDF} @ 20 EUR", Some("ASDF"));
        t("Assets:Cash     20 USD {10 ASDF} @    EUR", Some("ASDF"));
        t("Assets:Cash     20 USD {   ASDF} @ 20 EUR", Some("ASDF"));
        t("Assets:Cash     20 USD {   ASDF} @    EUR", Some("ASDF"));
        t("Assets:Cash        USD {10 ASDF} @ 20 EUR", Some("ASDF"));
        t("Assets:Cash        USD {10 ASDF} @    EUR", Some("ASDF"));
        t("Assets:Cash        USD {   ASDF} @ 20 EUR", Some("ASDF"));
        t("Assets:Cash        USD {   ASDF} @    EUR", Some("ASDF"));
    }

    fn check_single_group(
        currency: impl Into<Currency>,
        posting_strings: &[&str],
    ) -> Vec<RawPosting> {
        let posting = &postings_from_strings(posting_strings);
        let groups = group_and_fill_in_currencies(posting, &AccountBalances::new()).unwrap();
        assert_eq!(groups.len(), 1);
        let group = groups.into_iter().next().unwrap();
        assert_eq!(group.0, currency.into());
        group.1
    }

    fn check(posting_strings: &[&str]) -> GroupedPostings {
        let posting = &postings_from_strings(posting_strings);
        group_and_fill_in_currencies(posting, &AccountBalances::new()).unwrap()
    }

    #[test]
    fn test_filling_in_complete() {
        let group = check_single_group("USD", &["Assets:Cash 20 USD"]);
        insta::assert_json_snapshot!(group, @r###"
        [
          {
            "filename": null,
            "line": 2,
            "meta": [],
            "account": "Assets:Cash",
            "flag": null,
            "units": {
              "number": "20",
              "currency": "USD"
            },
            "price": null,
            "cost": null
          }
        ]
        "###);
    }

    #[test]
    fn test_filling_in_one_auto_posting() {
        let group = check_single_group("USD", &["Assets:Cash 20 USD", "Assets:Cash2"]);
        insta::assert_json_snapshot!(group, @r###"
        [
          {
            "filename": null,
            "line": 2,
            "meta": [],
            "account": "Assets:Cash",
            "flag": null,
            "units": {
              "number": "20",
              "currency": "USD"
            },
            "price": null,
            "cost": null
          },
          {
            "filename": null,
            "line": 3,
            "meta": [],
            "account": "Assets:Cash2",
            "flag": null,
            "units": {
              "number": null,
              "currency": "USD"
            },
            "price": null,
            "cost": null
          }
        ]
        "###);
    }

    #[test]
    fn test_filling_in_multiple_auto_postings() {
        let groups = check(&["Assets:Cash 20 USD", "Assets:Cash 20 EUR", "Assets:Cash2"]);
        insta::assert_json_snapshot!(groups, @r###"
        [
          [
            "USD",
            [
              {
                "filename": null,
                "line": 2,
                "meta": [],
                "account": "Assets:Cash",
                "flag": null,
                "units": {
                  "number": "20",
                  "currency": "USD"
                },
                "price": null,
                "cost": null
              },
              {
                "filename": null,
                "line": 4,
                "meta": [],
                "account": "Assets:Cash2",
                "flag": null,
                "units": {
                  "number": null,
                  "currency": "USD"
                },
                "price": null,
                "cost": null
              }
            ]
          ],
          [
            "EUR",
            [
              {
                "filename": null,
                "line": 3,
                "meta": [],
                "account": "Assets:Cash",
                "flag": null,
                "units": {
                  "number": "20",
                  "currency": "EUR"
                },
                "price": null,
                "cost": null
              },
              {
                "filename": null,
                "line": 4,
                "meta": [],
                "account": "Assets:Cash2",
                "flag": null,
                "units": {
                  "number": null,
                  "currency": "EUR"
                },
                "price": null,
                "cost": null
              }
            ]
          ]
        ]
        "###);
    }

    #[test]
    fn test_filling_cost() {
        let group = check_single_group("USD", &["Assets:Cash 20 USD", "Assets:Cash2 30 APL {}"]);
        insta::assert_json_snapshot!(group, @r###"
        [
          {
            "filename": null,
            "line": 2,
            "meta": [],
            "account": "Assets:Cash",
            "flag": null,
            "units": {
              "number": "20",
              "currency": "USD"
            },
            "price": null,
            "cost": null
          },
          {
            "filename": null,
            "line": 3,
            "meta": [],
            "account": "Assets:Cash2",
            "flag": null,
            "units": {
              "number": "30",
              "currency": "APL"
            },
            "price": null,
            "cost": {
              "number_per": null,
              "number_total": null,
              "currency": "USD",
              "date": null,
              "label": null,
              "merge": false
            }
          }
        ]
        "###);
    }

    #[test]
    fn test_filling_price() {
        let group = check_single_group("USD", &["Assets:Cash 20 USD", "Assets:Cash2 30 APL @"]);
        insta::assert_json_snapshot!(group, @r###"
        [
          {
            "filename": null,
            "line": 2,
            "meta": [],
            "account": "Assets:Cash",
            "flag": null,
            "units": {
              "number": "20",
              "currency": "USD"
            },
            "price": null,
            "cost": null
          },
          {
            "filename": null,
            "line": 3,
            "meta": [],
            "account": "Assets:Cash2",
            "flag": null,
            "units": {
              "number": "30",
              "currency": "APL"
            },
            "price": {
              "number": null,
              "currency": "USD"
            },
            "cost": null
          }
        ]
        "###);
    }

    #[test]
    fn test_filling_price_and_cost() {
        let group = check_single_group("USD", &["Assets:Cash 20 USD", "Assets:Cash2 30 APL {} @"]);
        insta::assert_json_snapshot!(group, @r###"
        [
          {
            "filename": null,
            "line": 2,
            "meta": [],
            "account": "Assets:Cash",
            "flag": null,
            "units": {
              "number": "20",
              "currency": "USD"
            },
            "price": null,
            "cost": null
          },
          {
            "filename": null,
            "line": 3,
            "meta": [],
            "account": "Assets:Cash2",
            "flag": null,
            "units": {
              "number": "30",
              "currency": "APL"
            },
            "price": {
              "number": null,
              "currency": "USD"
            },
            "cost": {
              "number_per": null,
              "number_total": null,
              "currency": "USD",
              "date": null,
              "label": null,
              "merge": false
            }
          }
        ]
        "###);
    }
}
