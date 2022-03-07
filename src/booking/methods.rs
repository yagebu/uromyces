use std::cmp::Reverse;

use crate::inventory::PositionWithCost;
use crate::types::{Amount, Booking, CostSpec, Decimal, IncompleteAmount, RawPosting};

use super::errors::{BookingError, BookingErrorKind};

/// Order in which existing might be reduced.
pub(super) enum ClosingOrder {
    Lifo,
    Fifo,
}

/// Booking methods that can be used to reduce inventories.
pub(super) enum BookingMethod {
    Average,
    Ordered(ClosingOrder),
    Strict,
}

impl BookingMethod {
    /// Turn a booking method option into one of the valid booking methods.
    pub fn from_option(b: Booking) -> Option<BookingMethod> {
        match b {
            Booking::Average => Some(BookingMethod::Average),
            Booking::Fifo => Some(BookingMethod::Ordered(ClosingOrder::Fifo)),
            Booking::Lifo => Some(BookingMethod::Ordered(ClosingOrder::Lifo)),
            Booking::Strict => Some(BookingMethod::Strict),
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
        ClosingOrder::Lifo => {
            matches.sort_by_key(|position| Reverse(position.cost.date));
        }
    };
    for position in matches {
        if remaining.is_sign_negative() {
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

/// Close the matching positions.
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
            todo!("implement strict booking method.")
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
