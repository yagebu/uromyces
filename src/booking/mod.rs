use hashbrown::HashMap;

use crate::inventory::Inventory;
use crate::ledgers::{Ledger, RawLedger};
use crate::tolerances::Tolerances;
use crate::types::{
    Account, Amount, Booking, Cost, CostSpec, Currency, Decimal, Entry, IncompleteAmount, Posting,
    RawEntry, RawPosting, RawTransaction, Transaction,
};

use self::currency_groups::group_and_fill_in_currencies;
use self::errors::{BookingError, BookingErrorKind};
use self::methods::{resolve_matches, BookingMethod};

mod currency_groups;
mod errors;
mod methods;

/// Contains information about the booking methods that are specified per account.
struct BookingMethods {
    map: HashMap<Account, Booking>,
    default_method: Booking,
}

impl BookingMethods {
    /// Collect all specified booking methods (and the default).
    fn from_ledger(raw_ledger: &RawLedger) -> BookingMethods {
        let mut map = HashMap::new();
        for e in &raw_ledger.entries {
            if let RawEntry::Open(entry) = e {
                if let Some(method) = entry.booking {
                    map.insert(entry.account.clone(), method);
                }
            }
        }
        BookingMethods {
            map,
            default_method: raw_ledger.options.booking_method,
        }
    }

    /// Get the booking method for a given account.
    fn get_account_booking_method(&self, account: &Account) -> &Booking {
        self.map.get(account).unwrap_or(&self.default_method)
    }
}

/// Find positions in the account balances that can be closed with the given postings.
///
/// This mutates the given list of raw postings in place.
fn close_positions(
    txn: &RawTransaction,
    balances: &HashMap<Account, Inventory>,
    postings: &mut Vec<RawPosting>,
    methods: &BookingMethods,
) -> Result<(), BookingError> {
    let mut additional_postings = Vec::new();

    for posting in &mut *postings {
        debug_assert!(posting.units.currency.is_some());
        if posting.cost.is_none() || posting.units.number.is_none() {
            continue;
        }

        let cost_spec = posting.cost.as_ref().unwrap(); // unwrap is safe: we just checked that the cost is not None
        let balance = balances.get(&posting.account);

        if let Some(bal) = balance {
            // number.is_some() is checked above, the currency should also be present
            let units = complete_amount(&posting.units)
                .expect("units to not have missing number or currency");
            let booking = methods.get_account_booking_method(&posting.account);
            let booking_method = BookingMethod::from_option(*booking);

            if booking_method.is_some() && bal.is_reduced_by(&units) {
                let matches = bal
                    .iter_with_cost()
                    .filter(|pos| units.currency == *pos.currency)
                    .filter(|pos| match &cost_spec.currency {
                        Some(currency) => currency == &pos.cost.currency,
                        None => true,
                    })
                    .filter(|pos| match &cost_spec.number_per {
                        Some(number) => number == &pos.cost.number,
                        None => true,
                    })
                    .filter(|pos| match &cost_spec.date {
                        Some(date) => date == &pos.cost.date,
                        None => true,
                    })
                    .filter(|pos| match &cost_spec.label {
                        Some(label) => pos.cost.label.iter().any(|v| v == label),
                        None => true,
                    })
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    return Err(BookingError::new(
                        posting,
                        BookingErrorKind::NoMatchesForReduction,
                    ));
                }
                additional_postings.append(&mut resolve_matches(
                    &booking_method.unwrap(), // unwrap is safe: we checked .is_some() above
                    posting,
                    matches,
                    &units,
                )?);
            } else if let Some(cost) = &mut posting.cost {
                cost.date.get_or_insert(txn.header.date);
            }
        } else if let Some(cost) = &mut posting.cost {
            cost.date.get_or_insert(txn.header.date);
        }
    }
    postings.append(&mut additional_postings);

    Ok(())
}

fn complete_amount(value: &IncompleteAmount) -> Result<Amount, BookingErrorKind> {
    let number = value.number.ok_or(BookingErrorKind::MissingAmountNumber)?;
    let currency = value
        .currency
        .as_ref()
        .expect("amount to have currency")
        .clone();

    Ok(Amount { number, currency })
}

fn complete_cost_spec(value: &CostSpec) -> Result<Cost, BookingErrorKind> {
    // TODO number_total
    let number = value
        .number_per
        .ok_or(BookingErrorKind::MissingCostNumber)?;
    let currency = value
        .currency
        .as_ref()
        .expect("cost to have currency")
        .clone();
    let date = value.date.expect("cost to have a date");

    Ok(Cost {
        number,
        currency,
        date,
        label: value.label.clone(),
    })
}

/// Compute the residual of a list of postings.
#[must_use]
pub fn compute_residual(postings: &[Posting]) -> Inventory {
    postings
        .iter()
        .map(crate::conversions::get_weight)
        .collect()
}

enum MissingValue {
    None(Amount, Option<Amount>, Option<Cost>),
    UnitsNumber(Option<Amount>, Option<Cost>),
    CostPerUnit(Amount, Option<Amount>),
    // CostTotal,
    PriceNumber(Amount, Option<Cost>),
}

/// Find which value might be missing in a posting.
fn find_missing_value(posting: &RawPosting) -> Result<MissingValue, BookingError> {
    let units = complete_amount(&posting.units);
    let price = posting.price.as_ref().map(complete_amount).transpose();
    let cost = posting.cost.as_ref().map(complete_cost_spec).transpose();

    match (units, price, cost) {
        (Ok(u), Ok(p), Ok(c)) => Ok(MissingValue::None(u, p, c)),
        (Err(..), Ok(p), Ok(c)) => Ok(MissingValue::UnitsNumber(p, c)),
        (Ok(u), Err(..), Ok(c)) => Ok(MissingValue::PriceNumber(u, c)),
        (Ok(u), Ok(p), Err(..)) => Ok(MissingValue::CostPerUnit(u, p)),
        _ => Err(BookingError::new(
            posting,
            BookingErrorKind::TooManyMissingNumbers,
        )),
    }
}

/// Interpolate and fill in missing numbers.
///
/// This turns `RawPosting`s into fully booked Postings. So this will error on any missing numbers
/// or currencies. The input postings should not have any missing currencies. Also all costs should
/// have a date already.
fn interpolate_and_fill_in_missing(
    postings: Vec<RawPosting>,
    group_currency: &Currency,
    tolerances: &Tolerances,
) -> Result<Vec<Posting>, BookingError> {
    let mut incomplete = None;
    let mut complete_postings = Vec::with_capacity(postings.len());

    for posting in postings {
        let missing_type = find_missing_value(&posting)?;
        if let MissingValue::None(units, price, cost) = missing_type {
            complete_postings.push(Posting {
                filename: posting.filename,
                line: posting.line,
                meta: posting.meta,
                account: posting.account,
                flag: posting.flag,
                units,
                price,
                cost,
            });
        } else {
            if incomplete.is_some() {
                return Err(BookingError::new(
                    &posting,
                    BookingErrorKind::TooManyMissingNumbers,
                ));
            }
            incomplete = Some((posting, missing_type));
        }
    }

    if let Some((posting, missing)) = incomplete {
        let residual = compute_residual(&complete_postings);
        let weight = if residual.is_empty() {
            Decimal::ZERO
        } else {
            debug_assert_eq!(residual.len(), 1);
            let pos = residual.iter().next().expect("missing residual");
            debug_assert!(pos.cost.is_none());
            debug_assert_eq!(pos.currency, group_currency);
            -*pos.number
        };

        let (units, price, cost) = match missing {
            MissingValue::UnitsNumber(price, cost) => {
                let number = if let Some(c) = &cost {
                    debug_assert_eq!(&c.currency, group_currency);
                    weight / c.number
                } else if let Some(p) = &price {
                    debug_assert_eq!(&p.currency, group_currency);
                    weight / p.number
                } else {
                    weight
                };
                let units = Amount {
                    currency: group_currency.clone(),
                    number: tolerances.quantize(group_currency, number),
                };
                (units, price, cost)
            }
            MissingValue::CostPerUnit(units, price) => {
                let mut cost_spec = posting.cost.clone().expect("should have a cost");
                cost_spec.number_per = Some(weight / units.number);
                let cost = complete_cost_spec(&cost_spec)
                    .expect("cost to not have missing number or currency");
                (units, price, Some(cost))
            }
            MissingValue::PriceNumber(units, cost) => {
                let price = Amount {
                    currency: group_currency.clone(),
                    number: weight / units.number,
                };
                (units, Some(price), cost)
            }
            MissingValue::None(units, price, cost) => (units, price, cost),
        };
        complete_postings.push(Posting {
            filename: posting.filename,
            line: posting.line,
            meta: posting.meta,
            account: posting.account,
            flag: posting.flag,
            units,
            price,
            cost,
        });
    }

    Ok(complete_postings)
}

/// Book and interpolate to fill in all missing values.
#[must_use]
pub(crate) fn book_entries(raw_ledger: RawLedger) -> Ledger {
    let booking_methods = BookingMethods::from_ledger(&raw_ledger);
    let mut balances = HashMap::new();

    // Closure to book a single transaction.
    let mut handle_txn = |txn: RawTransaction| -> Result<_, _> {
        let all_postings = {
            let mut all_postings = Vec::with_capacity(txn.postings.len());
            let tolerances = Tolerances::infer_from_raw(&txn.postings, &raw_ledger.options);

            let groups = group_and_fill_in_currencies(&txn.postings)?;
            for (currency, mut postings) in groups {
                close_positions(&txn, &balances, &mut postings, &booking_methods)?;
                all_postings.append(&mut interpolate_and_fill_in_missing(
                    postings,
                    &currency,
                    &tolerances,
                )?);
            }
            all_postings.sort_by_key(|p| p.line);
            all_postings
        };
        for posting in &all_postings {
            balances
                .raw_entry_mut()
                .from_key(&posting.account)
                .or_insert_with(|| (posting.account.clone(), Inventory::new()))
                .1
                .add_position(posting.units.clone(), posting.cost.clone());
        }
        Ok(Transaction {
            flag: txn.flag,
            header: txn.header,
            payee: txn.payee,
            narration: txn.narration,
            postings: all_postings,
        })
    };

    let mut entries = Vec::with_capacity(raw_ledger.entries.len());
    let mut errors = Vec::new();

    let mut ledger = Ledger::from_raw_empty_entries(&raw_ledger);

    for raw_entry in raw_ledger.entries {
        match raw_entry {
            RawEntry::Transaction(i) => {
                match handle_txn(i) {
                    Ok(txn) => entries.push(Entry::Transaction(txn)),
                    Err(err) => errors.push(err),
                };
            }
            RawEntry::Balance(i) => entries.push(Entry::Balance(i)),
            RawEntry::Close(i) => entries.push(Entry::Close(i)),
            RawEntry::Commodity(i) => entries.push(Entry::Commodity(i)),
            RawEntry::Custom(i) => entries.push(Entry::Custom(i)),
            RawEntry::Document(i) => entries.push(Entry::Document(i)),
            RawEntry::Event(i) => entries.push(Entry::Event(i)),
            RawEntry::Note(i) => entries.push(Entry::Note(i)),
            RawEntry::Open(i) => entries.push(Entry::Open(i)),
            RawEntry::Pad(i) => entries.push(Entry::Pad(i)),
            RawEntry::Price(i) => entries.push(Entry::Price(i)),
            RawEntry::Query(i) => entries.push(Entry::Query(i)),
        };
    }

    ledger.entries = entries;
    ledger.errors.append(&mut errors);
    ledger
}
