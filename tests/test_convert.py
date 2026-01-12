from __future__ import annotations

from datetime import date
from decimal import Decimal

from beancount.core import amount
from beancount.core import data

from uromyces import Amount
from uromyces import Balance
from uromyces._convert import beancount_to_uromyces
from uromyces._uromyces import Booking


def test_amount_constructor() -> None:
    ten = Decimal("10.00")
    assert Amount(ten, "USD").number == ten


def test_booking() -> None:
    assert Booking.NONE is Booking.NONE
    assert Booking.NONE.value == "NONE"
    assert Booking.STRICT.value == "STRICT"


def test_convert_beancount_to_uromyces() -> None:
    meta = {"filename": "<string>", "lineno": 0}
    bal = data.Balance(
        meta,
        date(2022, 12, 12),
        "Assets",
        amount.Amount(Decimal("10.00"), "USD"),
        Decimal("0.01"),
        None,
    )
    converted_bal = beancount_to_uromyces(bal)
    assert isinstance(converted_bal, Balance)
    assert converted_bal.tags == frozenset()
    assert converted_bal.links == frozenset()
    assert converted_bal.tolerance == Decimal("0.01")
