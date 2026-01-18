//! Implementations for the `__repr__` Python dunder method.

use crate::types::{Amount, Cost, CostLabel, CostSpec, Currency, Date, Decimal, RawAmount};

pub(crate) trait PyRepresentation {
    /// Build the Python string representation of the object.
    ///
    /// Should look like a Python expression that could be used to recreate the object or a string
    /// of the form `<..some description..>` otherwise.
    ///
    /// See also:
    /// <https://docs.python.org/3/reference/datamodel.html#object.__repr__>
    fn py_repr(&self) -> String;
}

impl<T: PyRepresentation> PyRepresentation for Option<T> {
    fn py_repr(&self) -> String {
        self.as_ref()
            .map_or_else(|| "None".to_string(), PyRepresentation::py_repr)
    }
}

impl PyRepresentation for bool {
    fn py_repr(&self) -> String {
        (if *self { "True" } else { "False" }).to_string()
    }
}

impl PyRepresentation for CostLabel {
    fn py_repr(&self) -> String {
        format!("'{self}'")
    }
}

impl PyRepresentation for Currency {
    fn py_repr(&self) -> String {
        format!("'{self}'")
    }
}

impl PyRepresentation for Date {
    fn py_repr(&self) -> String {
        format!(
            "datetime.date({}, {}, {})",
            self.year(),
            self.month(),
            self.day(),
        )
    }
}

impl PyRepresentation for Decimal {
    fn py_repr(&self) -> String {
        format!("Decimal('{self}')")
    }
}

impl PyRepresentation for Amount {
    fn py_repr(&self) -> String {
        format!(
            "Amount(number={}, currency={})",
            self.number.py_repr(),
            self.currency.py_repr()
        )
    }
}

impl PyRepresentation for RawAmount {
    fn py_repr(&self) -> String {
        format!(
            "RawAmount(number={}, currency={})",
            self.number.py_repr(),
            self.currency.py_repr()
        )
    }
}

impl PyRepresentation for Cost {
    fn py_repr(&self) -> String {
        format!(
            "Cost(number={}, currency={}, date={}, label={})",
            self.number.py_repr(),
            self.currency.py_repr(),
            self.date.py_repr(),
            self.label.py_repr()
        )
    }
}

impl PyRepresentation for CostSpec {
    fn py_repr(&self) -> String {
        format!(
            "CostSpec(number_per={}, number_total={}, currency={}, date={}, label={}, merge={})",
            self.number_per.py_repr(),
            self.number_total.py_repr(),
            self.currency.py_repr(),
            self.date.py_repr(),
            self.label.py_repr(),
            self.merge.py_repr(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{c, d};

    use super::*;

    #[test]
    fn test_various_types_repr() {
        assert_eq!(Decimal::new(100, 3).py_repr(), "Decimal('0.100')");
        assert_eq!(
            Date::from_ymd_opt(2012, 12, 31).unwrap().py_repr(),
            "datetime.date(2012, 12, 31)"
        );
        assert_eq!(c("EUR").py_repr(), "'EUR'");
        assert_eq!(Some(c("EUR")).py_repr(), "'EUR'");
        let none: Option<bool> = None;
        assert_eq!(none.py_repr(), "None");
        assert_eq!(false.py_repr(), "False");
        assert_eq!(true.py_repr(), "True");
    }

    #[test]
    fn test_cost_label_repr() {
        let label = CostLabel::from("test-label");
        assert_eq!(label.py_repr(), "'test-label'");
    }

    #[test]
    fn test_amount_repr() {
        let amount = Amount::new(d("123.45"), c("USD"));
        assert_eq!(
            amount.py_repr(),
            "Amount(number=Decimal('123.45'), currency='USD')"
        );
    }

    #[test]
    fn test_raw_amount_repr() {
        let raw_amount = RawAmount::new(Some(d("100.00")), Some(c("EUR")));
        assert_eq!(
            raw_amount.py_repr(),
            "RawAmount(number=Decimal('100.00'), currency='EUR')"
        );
        let raw_amount = RawAmount::new(None, Some(c("GBP")));
        assert_eq!(
            raw_amount.py_repr(),
            "RawAmount(number=None, currency='GBP')"
        );
        let raw_amount = RawAmount::new(Some(d("50.25")), None);
        assert_eq!(
            raw_amount.py_repr(),
            "RawAmount(number=Decimal('50.25'), currency=None)"
        );
        let raw_amount = RawAmount::new(None, None);
        assert_eq!(
            raw_amount.py_repr(),
            "RawAmount(number=None, currency=None)"
        );
    }

    #[test]
    fn test_cost_repr() {
        let date = Date::from_ymd_opt(2024, 1, 15).unwrap();
        let cost = Cost::new(d("10.50"), c("USD"), date, Some(CostLabel::from("lot1")));
        assert_eq!(
            cost.py_repr(),
            "Cost(number=Decimal('10.50'), currency='USD', date=datetime.date(2024, 1, 15), label='lot1')"
        );
        let cost = Cost::new(d("20.00"), c("EUR"), date, None);
        assert_eq!(
            cost.py_repr(),
            "Cost(number=Decimal('20.00'), currency='EUR', date=datetime.date(2024, 1, 15), label=None)"
        );
    }

    #[test]
    fn test_cost_spec_repr() {
        let date = Date::from_ymd_opt(2024, 6, 30).unwrap();
        let cost_spec = CostSpec {
            number_per: Some(d("5.00")),
            number_total: Some(d("100.00")),
            currency: Some(c("USD")),
            date: Some(date),
            label: Some(CostLabel::from("batch-1")),
            merge: true,
        };
        assert_eq!(
            cost_spec.py_repr(),
            "CostSpec(number_per=Decimal('5.00'), number_total=Decimal('100.00'), currency='USD', date=datetime.date(2024, 6, 30), label='batch-1', merge=True)"
        );

        let cost_spec = CostSpec {
            number_per: None,
            number_total: None,
            currency: None,
            date: None,
            label: None,
            merge: false,
        };
        assert_eq!(
            cost_spec.py_repr(),
            "CostSpec(number_per=None, number_total=None, currency=None, date=None, label=None, merge=False)"
        );
    }
}
