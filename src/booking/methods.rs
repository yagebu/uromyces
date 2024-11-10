use std::cmp::Reverse;

use crate::inventory::PositionWithCost;
use crate::types::{Amount, Booking, CostSpec, Decimal, IncompleteAmount, RawPosting};

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

/// Close positions using either FIFO (first-in-first-out) or LIFO (last-in-first-out).
fn resolve_ordered(
    posting_units: &Amount,
    mut matches: Vec<PositionWithCost>,
    order: &ClosingOrder,
) -> Result<Vec<(IncompleteAmount, CostSpec)>, BookingErrorKind> {
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
            IncompleteAmount {
                number: Some(reduced),
                currency: Some(position.currency.clone()),
            },
            cost.into(),
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
) -> Result<Vec<(IncompleteAmount, CostSpec)>, BookingErrorKind> {
    let sign_positive = posting_units.number.is_sign_positive();
    let mut remaining = posting_units.number.abs();

    if matches.len() > 1 {
        // If the total requested to reduce matches the sum of all the
        // ambiguous postings, match against all of them.
        let sum_matches: Decimal = matches.iter().map(|p| p.number).sum();
        if sum_matches == -posting_units.number {
            let resolved: Vec<(IncompleteAmount, CostSpec)> = matches
                .iter()
                .map(|position| {
                    let mut reduced = std::cmp::min(position.number.abs(), remaining);
                    remaining -= reduced;
                    reduced.set_sign_positive(sign_positive);
                    (
                        IncompleteAmount {
                            number: Some(reduced),
                            currency: Some(position.currency.clone()),
                        },
                        position.cost.into(),
                    )
                })
                .collect();
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
                IncompleteAmount {
                    number: Some(reduced),
                    currency: Some(position.currency.clone()),
                },
                position.cost.into(),
            )])
        }
    }
}

/// Close the matching positions.
///
/// Mutates the given posting in place and returns additional postings (can be empty) if needed.
pub(super) fn resolve_matches(
    method: &BookingMethod,
    posting: &mut RawPosting,
    matches: Vec<PositionWithCost>,
    units: &Amount,
) -> Result<Vec<RawPosting>, BookingError> {
    // If this turns up more than one match, we add postings.
    let mut additional_postings = vec![];
    // We mutate the first match directly and then clone the posting for the additional ones.
    let mut initial = true;

    // Add closing position to posting.
    let mut add_closing_position = |p: &mut RawPosting, units: IncompleteAmount, cost: CostSpec| {
        if initial {
            p.units = units;
            p.cost = Some(cost);
            initial = false;
        } else {
            let mut pos = p.clone();
            pos.units = units;
            pos.cost = Some(cost);
            additional_postings.push(pos);
        }
    };

    match method {
        BookingMethod::Ordered(order) => {
            let resolved = resolve_ordered(units, matches, order)
                .map_err(|kind| BookingError::new(posting, kind))?;
            for (units, cost) in resolved {
                add_closing_position(posting, units, cost);
            }
        }
        BookingMethod::Strict => {
            let resolved =
                resolve_strict(units, &matches).map_err(|kind| BookingError::new(posting, kind))?;
            for (units, cost) in resolved {
                add_closing_position(posting, units, cost);
            }
        }
        BookingMethod::Average => {
            return Err(BookingError::new(
                posting,
                BookingErrorKind::UnsupportedAverageBooking,
            ))
        }
    }

    Ok(additional_postings)
}
