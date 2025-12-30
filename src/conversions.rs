//! Convert amounts and positions.
use crate::types::{Amount, Posting};

/// Get the weight of a posting.
///
/// The weight of the posting is the following:
/// - if the posting has a cost, multiply the units by the cost
/// - if the posting has a price, multiply the units by the price
/// - units otherwise
pub fn get_weight(posting: &Posting) -> Amount {
    if let Some(cost) = &posting.cost {
        Amount::new(cost.number * posting.units.number, cost.currency.clone())
    } else if let Some(price) = &posting.price {
        Amount::new(price.number * posting.units.number, price.currency.clone())
    } else {
        posting.units.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_utils::a, types::Filename};

    use super::*;

    #[test]
    fn test_get_weight_units() {
        let pos = Posting::new_simple(
            Filename::new_dummy("string"),
            "Assets:Cash".into(),
            a("10 EUR"),
        );
        assert_eq!(get_weight(&pos), a("10 EUR"));
    }
}
