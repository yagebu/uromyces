from __future__ import annotations

from datetime import date
from decimal import Decimal

import pytest
from beancount.core import amount
from beancount.core import data

from uromyces._convert import beancount_to_uromyces
from uromyces.uromyces import Amount
from uromyces.uromyces import Balance
from uromyces.uromyces import Booking
from uromyces.uromyces import EntryHeader


def test_amount_constructor() -> None:
    ten = Decimal("10.00")
    assert Amount(ten, "USD").number == ten


def test_booking() -> None:
    assert Booking.NONE is Booking.NONE
    assert Booking.NONE.value == "NONE"
    assert Booking.STRICT.value == "STRICT"


def test_entry_header_constructor() -> None:
    header = EntryHeader(
        {"filename": "asdf", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    assert header.tags == frozenset(("asdf",))
    # not an absolute path
    assert header.filename is None
    header = EntryHeader(
        {"filename": "/home", "lineno": 0, "key": "string"},
        date(2022, 12, 12),
        frozenset("asdf"),
    )
    assert header.filename == "/home"
    assert header["filename"] == "/home"
    assert header["lineno"] == 0
    assert header["key"] == "string"
    assert len(header) == 3  # noqa: PLR2004
    with pytest.raises(KeyError):
        header["asdf"]

    header = EntryHeader(
        {"filename": "/home", "lineno": 0, "__implicit_prices": "string"},
        date(2022, 12, 12),
        frozenset("asdf"),
    )


def test_convert_beancount_to_uromyces() -> None:
    meta = {"filename": "asdf", "lineno": 0}
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
