use std::cmp::Reverse;

use crate::inventory::{Inventory, InventoryPosition, PositionWithCost};
use crate::types::{Amount, Booking, Cost, Decimal, RawPosting};

use super::errors::{BookingError, BookingErrorKind};

/// Order in which existing might be reduced.
pub(super) enum ClosingOrder {
    Fifo,
    Hifo,
    Lifo,
}

/// Booking methods that can be used to reduce inventories.
pub(super) enum BookingMethod {
    Average,
    Ordered(ClosingOrder),
    Strict,
}

impl BookingMethod {
    /// Turn a booking method option into one of the valid booking methods.
    pub fn from_option(b: Booking) -> Option<Self> {
        match b {
            Booking::Average => Some(Self::Average),
            Booking::Fifo => Some(Self::Ordered(ClosingOrder::Fifo)),
            Booking::Hifo => Some(Self::Ordered(ClosingOrder::Hifo)),
            Booking::Lifo => Some(Self::Ordered(ClosingOrder::Lifo)),
            Booking::Strict => Some(Self::Strict),
            Booking::None => None,
        }
    }
}

/// Close positions using either
/// - FIFO (first-in-first-out)
/// - LIFO (last-in-first-out)
/// - HIFO (highest-in-first-out)
fn resolve_ordered(
    posting_units: &Amount,
    mut matches: Vec<PositionWithCost>,
    order: &ClosingOrder,
) -> Result<Vec<(Amount, Cost)>, BookingErrorKind> {
    let mut resolved = vec![];

    let sign_positive = posting_units.number.is_sign_positive();
    let mut remaining = posting_units.number.abs();

    match order {
        ClosingOrder::Fifo => {
            matches.sort_by_key(|position| position.cost.date);
        }
        ClosingOrder::Hifo => {
            matches.sort_by_key(|position| Reverse(position.cost.number));
        }
        ClosingOrder::Lifo => {
            matches.sort_by_key(|position| Reverse(position.cost.date));
        }
    };
    for position in matches {
        // We only need to continue if we have a positive non-zero amount remaining.
        if remaining.is_sign_negative() || remaining.is_zero() {
            break;
        }

        let cost = position.cost;
        let mut reduced = std::cmp::min(position.number.abs(), remaining);
        remaining -= reduced;
        reduced.set_sign_positive(sign_positive);

        resolved.push((
            Amount::new(reduced, position.currency.clone()),
            cost.clone(),
        ));
    }

    if remaining > Decimal::ZERO {
        Err(BookingErrorKind::InsufficientLots)
    } else {
        Ok(resolved)
    }
}

fn resolve_strict(
    posting_units: &Amount,
    matches: &[PositionWithCost],
) -> Result<Vec<(Amount, Cost)>, BookingErrorKind> {
    let sign_positive = posting_units.number.is_sign_positive();
    let mut remaining = posting_units.number.abs();

    if matches.len() > 1 {
        // If the total requested to reduce matches the sum of all the
        // ambiguous postings, match against all of them.
        let sum_matches: Decimal = matches.iter().map(|p| p.number).sum();
        if sum_matches == -posting_units.number {
            let resolved = matches
                .iter()
                .map(|position| {
                    let mut reduced = std::cmp::min(position.number.abs(), remaining);
                    remaining -= reduced;
                    reduced.set_sign_positive(sign_positive);
                    (
                        Amount::new(reduced, position.currency.clone()),
                        position.cost.clone(),
                    )
                })
                .collect::<Vec<_>>();
            Ok(resolved)
        } else {
            Err(BookingErrorKind::AmbiguousMatches)
        }
    } else {
        let position = &matches[0];
        let mut reduced = std::cmp::min(position.number.abs(), remaining);

        remaining -= reduced;
        reduced.set_sign_positive(sign_positive);

        if remaining > Decimal::ZERO {
            Err(BookingErrorKind::InsufficientLots)
        } else {
            Ok(vec![(
                Amount::new(reduced, position.currency.clone()),
                position.cost.clone(),
            )])
        }
    }
}

/// Resolves matching positions.
pub(super) fn resolve_matches(
    method: &BookingMethod,
    posting: &mut RawPosting,
    matches: Vec<PositionWithCost>,
    units: &Amount,
) -> Result<Vec<(Amount, Cost)>, BookingError> {
    debug_assert!(posting.cost.is_some());

    match method {
        BookingMethod::Ordered(order) => {
            resolve_ordered(units, matches, order).map_err(|kind| BookingError::new(posting, kind))
        }
        BookingMethod::Strict => {
            resolve_strict(units, &matches).map_err(|kind| BookingError::new(posting, kind))
        }
        BookingMethod::Average => Err(BookingError::new(
            posting,
            BookingErrorKind::UnsupportedAverageBooking,
        )),
    }
}

/// Close the matching positions.
///
/// Mutates the given posting in place and returns additional postings (can be empty) if needed.
pub(super) fn close_with_resolved_matches(
    posting: &mut RawPosting,
    balance: &mut Inventory,
    resolved_matches: Vec<(Amount, Cost)>,
) -> Vec<RawPosting> {
    debug_assert!(posting.cost.is_some());
    // If this turns up more than one match, we add postings.
    let mut additional_postings = vec![];
    // We mutate the first match directly and then clone the posting for the additional ones.
    let mut initial = true;

    for (units, cost) in resolved_matches {
        let pos = InventoryPosition {
            number: &units.number,
            currency: &units.currency,
            cost: &Some(cost.clone()),
        };
        balance.add_position(&pos);
        if initial {
            posting.units = units.into();
            posting.cost = Some(cost.into());
            initial = false;
        } else {
            let mut additional_posting = posting.clone();
            additional_posting.units = units.into();
            additional_posting.cost = Some(cost.into());
            additional_postings.push(additional_posting);
        }
    }

    debug_assert!(posting.units.number.is_some());
    debug_assert!(posting.units.currency.is_some());
    debug_assert!(posting.cost.is_some());
    // If no error occured, the posting and any additional postings should have a cost.
    debug_assert!(additional_postings.iter().all(|p| p.cost.is_some()));
    additional_postings
}
