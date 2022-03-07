use crate::types::{Amount, Posting};

/// Get the weight of a posting.
pub fn get_weight(posting: &Posting) -> Amount {
    if let Some(cost) = &posting.cost {
        Amount {
            number: cost.number * posting.units.number,
            currency: cost.currency.clone(),
        }
    } else if let Some(price) = &posting.price {
        Amount {
            number: price.number * posting.units.number,
            currency: price.currency.clone(),
        }
    } else {
        posting.units.clone()
    }
}
