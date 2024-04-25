from __future__ import annotations

from datetime import date
from decimal import Decimal
from typing import TYPE_CHECKING

import pytest
from beancount.core import amount  # type: ignore[import-untyped]

from uromyces import Amount
from uromyces import Balance
from uromyces import Close
from uromyces import Commodity
from uromyces import Cost
from uromyces import Custom
from uromyces import Document
from uromyces import EntryHeader
from uromyces import Event
from uromyces import Note
from uromyces import Open
from uromyces import Pad
from uromyces import Posting
from uromyces import Price
from uromyces import Query
from uromyces import Transaction

if TYPE_CHECKING:
    from uromyces import Entry


def test_amount() -> None:
    amt = Amount(Decimal("10.00"), "USD")
    amt2 = Amount(Decimal("10"), "USD")
    assert amt == amt2
    assert amt != Amount(Decimal("11"), "USD")
    assert hash(amt) == hash(amt2)


def test_cost() -> None:
    cost = Cost(Decimal("10.00"), "USD", date(2000, 1, 1), None)
    cost2 = Cost(Decimal("10.00"), "USD", date(2000, 1, 1), None)
    assert cost == cost2
    assert cost != Cost(Decimal("10.00"), "USD", date(2000, 1, 2), None)
    assert hash(cost) == hash(cost2)


def test_equals() -> None:
    assert Amount(Decimal("10.00"), "USD") == Amount(Decimal("10"), "USD")
    header = EntryHeader(
        {"filename": "asdf", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    assert Balance(
        header,
        "Assets:Cash",
        Amount(Decimal("10.00"), "USD"),
        None,
    ) == Balance(
        header,
        "Assets:Cash",
        Amount(Decimal("10"), "USD"),
        None,
    )


def test_balance() -> None:
    header = EntryHeader(
        {"filename": "asdf", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    balance = Balance(
        header,
        "Assets:Cash",
        Amount(Decimal("10.00"), "USD"),
        None,
    )
    assert balance.account == "Assets:Cash"

    bal_replaced_account = balance._replace(account="Assets:Other")
    assert bal_replaced_account.account == "Assets:Other"
    assert bal_replaced_account.amount.number == Decimal("10.00")

    bal_replaced_amount = balance._replace(
        amount=Amount(Decimal("20.00"), "USD"),
    )
    assert bal_replaced_amount.amount.number == Decimal("20.00")

    bal_replace_multi = balance._replace(
        amount=Amount(Decimal("20.00"), "USD"),
        account="Assets:Other",
    )
    assert bal_replace_multi.amount.number == Decimal("20.00")
    assert bal_replace_multi.account == "Assets:Other"

    assert balance._replace(meta={"key": "value"}).meta["key"] == "value"


HEADER = EntryHeader(
    {"filename": "asdf", "lineno": 0},
    date(2022, 12, 12),
    {"a-tag"},
    {"a-link"},
)


@pytest.mark.parametrize(
    "entry",
    [
        Balance(HEADER, "A:C", Amount(Decimal("1"), "USD"), None),
        Close(HEADER, "A:C"),
        Commodity(HEADER, "USD"),
        Custom(HEADER, "custom-type", []),
        Document(HEADER, "Assets:Cash", "/path/to/file"),
        Event(HEADER, "event-type", "event-name"),
        Note(HEADER, "A:C", "account note"),
        Open(HEADER, "A:C", ["USD"], None),
        Pad(HEADER, "A:C", "A:Source"),
        Price(HEADER, "A:C", Amount(Decimal("1"), "USD")),
        Query(HEADER, "name", "query"),
        Transaction(HEADER, "*", "payee", "narration", []),
    ],
)
def test_entry_types(entry: Entry) -> None:
    assert hash(entry)
    assert entry.date == date(2022, 12, 12)
    assert entry.links == {"a-link"}
    assert entry.links == {"a-link"}
    assert entry._replace(date=date(2022, 12, 13)).date == date(2022, 12, 13)
    assert entry._replace(tags={"another-tag"}).tags == {"another-tag"}
    assert entry._replace(links={"another-link"}).links == {"another-link"}

    with pytest.raises(TypeError, match="takes 0 positional arguments"):
        assert entry._replace("")  # type: ignore[arg-type,misc,union-attr]


def test_document() -> None:
    header = EntryHeader(
        {"filename": "asdf", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    document = Document(header, "Assets:Cash", "/path/to/file")
    assert document == Document(
        header, account="Assets:Cash", filename="/path/to/file"
    )
    assert document.filename == "/path/to/file"
    assert document._replace(filename="/other/path").filename == "/other/path"
    assert document._replace(tags={"newtag"}).tags == {"newtag"}


def test_transaction() -> None:
    header = EntryHeader(
        {"filename": "asdf", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    transaction = Transaction(header, "*", "payee", "narration", [])
    assert transaction.flag == "*"
    assert transaction.payee == "payee"
    assert transaction.narration == "narration"
    assert transaction.postings == []

    units = Amount(Decimal("10.00"), "USD")
    posting = Posting("Assets:A1", units, None, None, None, None)
    postings = [posting]
    assert posting == Posting("Assets:A1", units)
    assert posting == Posting(
        "Assets:A1", amount.Amount(Decimal("10.00"), "USD")
    )
    t = transaction._replace(postings=postings)
    assert t.postings == postings
