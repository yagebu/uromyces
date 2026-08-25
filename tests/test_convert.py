from __future__ import annotations

from datetime import date
from decimal import Decimal

from beancount.core import amount
from beancount.core import data

from uromyces import Amount
from uromyces import Balance
from uromyces import Transaction
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
    converted_bal = Balance._from_beancount(bal)
    assert isinstance(converted_bal, Balance)
    assert converted_bal.meta == meta
    assert converted_bal.account == "Assets"
    assert converted_bal.tags == frozenset()
    assert converted_bal.links == frozenset()
    assert converted_bal.tolerance == Decimal("0.01")


def test_convert_transaction_with_tuple_postings() -> None:
    """`postings` should convert whether it's a list or any other iterable."""
    meta = {"filename": "<string>", "lineno": 0}
    posting = data.Posting(
        "Assets:Cash",
        amount.Amount(Decimal("1.00"), "USD"),
        None,
        None,
        None,
        None,
    )
    txn = data.Transaction(
        meta,
        date(2022, 12, 12),
        "*",
        None,
        "narration",
        frozenset(),
        frozenset(),
        [posting],
    )
    txn = txn._replace(postings=tuple(txn.postings))  # type: ignore[arg-type]  # ty:ignore[invalid-argument-type]

    converted_txn = Transaction._from_beancount(txn)
    assert isinstance(converted_txn, Transaction)
    assert len(converted_txn.postings) == 1
    assert converted_txn.postings[0].account == "Assets:Cash"
