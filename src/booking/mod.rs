//! Booking - finding matching positions when reducing inventories
use hashbrown::HashMap;

use crate::conversions::get_weight;
use crate::inventory::Inventory;
use crate::ledgers::{Ledger, RawLedger};
use crate::tolerances::Tolerances;
use crate::types::{
    Account, Amount, Booking, Cost, CostSpec, Currency, Date, Decimal, Entry, IncompleteAmount,
    Posting, RawEntry, RawPosting, RawTransaction, Transaction,
};

use currency_groups::group_and_fill_in_currencies;
use errors::{BookingError, BookingErrorKind};
use methods::{BookingMethod, close_with_resolved_matches, resolve_matches};

mod currency_groups;
mod errors;
mod methods;
#[cfg(test)]
mod tests;

/// Contains information about the booking methods that are specified per account.
///
/// This is constructed from a [`RawLedger`] and allows a quick lookup to get the per-account booking
/// method (or the default, set per Beancount option).
struct BookingMethods {
    map: HashMap<Account, Booking>,
    default_method: Booking,
}

impl BookingMethods {
    /// Collect all specified booking methods (and the default).
    ///
    /// This iterates over all open directives in the ledger and collects the booking methods (if
    /// specified). The default booking method to use as a fallback is read from the options.
    fn from_ledger(raw_ledger: &RawLedger) -> Self {
        let mut map = HashMap::new();
        for e in &raw_ledger.entries {
            if let RawEntry::Open(entry) = e
                && let Some(method) = entry.booking
            {
                map.insert(entry.account.clone(), method);
            }
        }
        Self {
            map,
            default_method: raw_ledger.options.booking_method,
        }
    }

    /// Get the booking method for a given account.
    fn get_account_booking_method(&self, account: &Account) -> &Booking {
        self.map.get(account).unwrap_or(&self.default_method)
    }
}

type AccountBalances = HashMap<Account, Inventory>;

/// Find positions in the account balances that can be closed with the given postings.
///
/// This mutates the given list of raw postings in place.
fn close_positions(
    balances: &AccountBalances,
    postings: &mut Vec<RawPosting>,
    methods: &BookingMethods,
) -> Result<(), BookingError> {
    let mut additional_postings = Vec::new();
    // We keep local balances to allow multiple reductions to the same account in one
    // Transaction, while ensuring the balances do not get partially updated if booking fails
    // half-way through a Transaction.
    let mut local_balances = AccountBalances::new();

    for posting in postings.iter_mut() {
        debug_assert!(posting.units.currency.is_some());

        let Some(cost) = &posting.cost else {
            continue;
        };
        let Some(units_number) = posting.units.number else {
            continue;
        };
        let Some(units_currency) = &posting.units.currency else {
            continue;
        };

        // Get the value from our local balances (or insert, getting the value from the passed in
        // balances).
        let balance = local_balances
            .raw_entry_mut()
            .from_key(&posting.account)
            .or_insert_with(|| {
                (
                    posting.account.clone(),
                    balances.get(&posting.account).cloned().unwrap_or_default(),
                )
            })
            .1;

        let units = Amount::new(units_number, units_currency.clone());
        let booking = methods.get_account_booking_method(&posting.account);
        let Some(booking_method) = BookingMethod::from_option(*booking) else {
            continue;
        };

        if balance.is_reduced_by(&units) {
            let matches = balance
                .iter_with_cost()
                .filter(|pos| {
                    units.currency == *pos.currency
                        && cost
                            .currency
                            .as_ref()
                            .is_none_or(|c| c == &pos.cost.currency)
                        && cost
                            .number_per
                            .as_ref()
                            .is_none_or(|n| n == &pos.cost.number)
                        && cost.date.as_ref().is_none_or(|d| d == &pos.cost.date)
                        && cost
                            .label
                            .as_ref()
                            .is_none_or(|l| pos.cost.label.iter().any(|v| v == l))
                })
                .collect::<Vec<_>>();
            if matches.is_empty() {
                return Err(BookingErrorKind::NoMatchesForReduction.with_posting(posting));
            }
            let resolved_matches = resolve_matches(&booking_method, posting, matches, &units)?;
            let mut resolved = close_with_resolved_matches(posting, balance, resolved_matches);
            additional_postings.append(&mut resolved);
        }
    }
    postings.append(&mut additional_postings);

    Ok(())
}

/// Try to complete an incomplete amount.
fn complete_amount(value: &IncompleteAmount) -> Result<Amount, BookingErrorKind> {
    let number = value.number.ok_or(BookingErrorKind::MissingAmountNumber)?;
    let currency = value
        .currency
        .as_ref()
        .expect("amount to have currency")
        .clone();

    Ok(Amount::new(number, currency))
}

/// Try to complete a cost spec to a cost.
fn complete_cost_spec(
    cost: &CostSpec,
    date: Date,
    units_number: Option<Decimal>,
) -> Result<Cost, BookingErrorKind> {
    let number = if let Some(number_total) = cost.number_total {
        let units_number = units_number
            .ok_or(BookingErrorKind::MissingAmountNumber)?
            .abs();
        let mut total = number_total;
        if let Some(number_per) = cost.number_per {
            total += number_per * units_number;
        }
        total / units_number
    } else {
        cost.number_per.ok_or(BookingErrorKind::MissingCostNumber)?
    };
    let currency = cost
        .currency
        .as_ref()
        .expect("cost to have currency")
        .clone();

    Ok(Cost {
        number,
        currency,
        date: cost.date.unwrap_or(date),
        label: cost.label.clone(),
    })
}

/// The variants of where a number might be missing in a posting.
enum MissingNumber {
    /// Nothing missing, all values are present
    None(Amount, Option<Amount>, Option<Cost>),
    /// The number of the units is missing, price and cost are present.
    UnitsNumber(Option<Amount>, Option<Cost>),
    /// The per-unit cost is missing, units and price are present.
    CostPerUnit(Amount, Option<Amount>),
    /// The number of the price is missing, units and cost are present.
    PriceNumber(Amount, Option<Cost>),
    // TODO: CostTotal,
}

/// Find which value might be missing in a posting.
fn find_missing_value(posting: &RawPosting, date: Date) -> Result<MissingNumber, BookingError> {
    let units = complete_amount(&posting.units);
    let price = posting.price.as_ref().map(complete_amount).transpose();
    let cost = posting
        .cost
        .as_ref()
        .map(|c| complete_cost_spec(c, date, posting.units.number))
        .transpose();

    match (units, price, cost) {
        (Ok(u), Ok(p), Ok(c)) => Ok(MissingNumber::None(u, p, c)),
        (Err(..), Ok(p), Ok(c)) => Ok(MissingNumber::UnitsNumber(p, c)),
        (Ok(u), Err(..), Ok(c)) => Ok(MissingNumber::PriceNumber(u, c)),
        (Ok(u), Ok(p), Err(..)) => Ok(MissingNumber::CostPerUnit(u, p)),
        _ => Err(BookingErrorKind::TooManyMissingNumbers.with_posting(posting)),
    }
}

/// Interpolate and fill in missing numbers.
///
/// This turns `RawPosting`s into fully booked Postings. So this will error on any missing numbers
/// or currencies. The input postings should not have any missing currencies.
///
/// Use the provided (entry) date for all costs that do not have an explicit date.
fn interpolate_and_fill_in_missing(
    postings: Vec<RawPosting>,
    group_currency: &Currency,
    tolerances: &Tolerances,
    date: Date,
) -> Result<Vec<Posting>, BookingError> {
    let mut incomplete = None;
    let mut complete_postings = Vec::with_capacity(postings.len());

    for posting in postings {
        let missing_type = find_missing_value(&posting, date)?;
        if let MissingNumber::None(units, price, cost) = missing_type {
            complete_postings.push(posting.complete(units, price, cost));
        } else {
            if incomplete.is_some() {
                return Err(BookingErrorKind::TooManyMissingNumbers.with_posting(&posting));
            }
            incomplete = Some((posting, missing_type));
        }
    }

    if let Some((posting, missing)) = incomplete {
        // Compute the residual of the complete postings, which must all have a weight in `group_currency`).
        let weight = -complete_postings
            .iter()
            .map(get_weight)
            .map(|weight| {
                assert_eq!(&weight.currency, group_currency);
                weight.number
            })
            .sum::<Decimal>();

        if let Some((units, price, cost)) = match missing {
            MissingNumber::UnitsNumber(price, cost) => {
                if weight.is_zero() {
                    None
                } else {
                    let number = if let Some(c) = &cost {
                        debug_assert_eq!(&c.currency, group_currency);
                        weight / c.number
                    } else if let Some(p) = &price {
                        debug_assert_eq!(&p.currency, group_currency);
                        weight / p.number
                    } else {
                        weight
                    };
                    let units = Amount::new(
                        tolerances.quantize(group_currency, number),
                        group_currency.clone(),
                    );

                    Some((units, price, cost))
                }
            }
            MissingNumber::CostPerUnit(units, price) => {
                let mut cost_spec = posting.cost.clone().expect("should have a cost");
                if units.number.is_zero() {
                    None
                } else {
                    cost_spec.number_per = Some(weight / units.number);
                    let cost = complete_cost_spec(&cost_spec, date, posting.units.number)
                        .expect("cost to not have missing number or currency");
                    Some((units, price, Some(cost)))
                }
            }
            MissingNumber::PriceNumber(units, cost) => {
                let price = Amount::new(weight / units.number, group_currency.clone());
                Some((units, Some(price), cost))
            }
            MissingNumber::None(units, price, cost) => Some((units, price, cost)),
        } {
            complete_postings.push(posting.complete(units, price, cost));
        }
    }

    Ok(complete_postings)
}

/// Update the running balances for all postings of a booked transaction.
fn update_running_balances(balances: &mut AccountBalances, transaction: &Transaction) {
    for posting in &transaction.postings {
        balances
            .raw_entry_mut()
            .from_key(&posting.account)
            .or_insert_with(|| (posting.account.clone(), Inventory::new()))
            .1
            .add_position(posting);
    }
}

/// Book and interpolate to fill in all missing values.
#[must_use]
pub(crate) fn book_entries(raw_ledger: RawLedger) -> (Ledger, AccountBalances) {
    let booking_methods = BookingMethods::from_ledger(&raw_ledger);
    let mut balances = AccountBalances::new();

    // Closure to book a single transaction.
    let handle_txn = |balances: &AccountBalances, txn: RawTransaction| -> Result<Transaction, _> {
        let booked_postings = {
            let mut booked_postings = Vec::with_capacity(txn.postings.len());
            let tolerances = Tolerances::infer_from_raw(&txn.postings, &raw_ledger.options);

            let groups = group_and_fill_in_currencies(&txn.postings, balances)?;
            for (currency, mut postings) in groups {
                close_positions(balances, &mut postings, &booking_methods)?;
                booked_postings.append(&mut interpolate_and_fill_in_missing(
                    postings,
                    &currency,
                    &tolerances,
                    txn.date,
                )?);
            }
            booked_postings.sort_by_key(|p| p.meta.lineno);
            booked_postings
        };
        Ok(txn.complete(booked_postings))
    };

    let mut entries = Vec::with_capacity(raw_ledger.entries.len());
    let mut errors = Vec::new();

    let mut ledger = Ledger::from_raw_empty_entries(&raw_ledger);

    for raw_entry in raw_ledger.entries {
        match raw_entry {
            RawEntry::RawTransaction(i) => match handle_txn(&balances, i) {
                Ok(txn) => {
                    update_running_balances(&mut balances, &txn);
                    entries.push(Entry::Transaction(txn));
                }
                Err(err) => errors.push(err),
            },
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
        }
    }

    ledger.entries = entries;
    ledger.errors.append(&mut errors);
    (ledger, balances)
}
