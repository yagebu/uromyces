from __future__ import annotations

from collections.abc import Mapping
from datetime import date
from decimal import Decimal
from typing import TYPE_CHECKING

import pytest
from beancount.core import amount
from beancount.core import data

from uromyces import Amount
from uromyces import Balance
from uromyces import Close
from uromyces import Commodity
from uromyces import Cost
from uromyces import Custom
from uromyces import CustomValue
from uromyces import Document
from uromyces import EntryMeta
from uromyces import Event
from uromyces import Note
from uromyces import Open
from uromyces import Pad
from uromyces import Posting
from uromyces import Price
from uromyces import Query
from uromyces import Transaction
from uromyces._convert import beancount_to_uromyces

if TYPE_CHECKING:
    from uromyces import Directive
    from uromyces import Ledger


def test_amount() -> None:
    amt = Amount(Decimal("10.00"), "USD")
    amt2 = Amount(Decimal(10), "USD")
    assert amt == amt2
    assert repr(amt) == "Amount(number=Decimal('10.00'), currency='USD')"
    assert str(amt) == "10.00 USD"
    assert repr(amt2) == "Amount(number=Decimal('10'), currency='USD')"
    assert amt != Amount(Decimal(11), "USD")
    assert hash(amt) == hash(amt2)


def test_amount_decimal_edge_cases() -> None:
    amt = Amount(Decimal("1E-20"), "USD")
    assert amt.number == Decimal("1E-20")
    with pytest.raises(ValueError, match=r"exceeds the maximum precision"):
        Amount(Decimal("1E-50"), "USD")


def test_cost() -> None:
    cost = Cost(Decimal("10.00"), "USD", date(2000, 1, 1), None)
    cost2 = Cost(Decimal("10.00"), "USD", date(2000, 1, 1), None)
    cost_label = Cost(Decimal("10.00"), "USD", date(2000, 1, 1), "label")
    assert cost == cost2
    assert cost != Cost(Decimal("10.00"), "USD", date(2000, 1, 2), None)
    assert hash(cost) == hash(cost2)

    assert (
        repr(cost) == "Cost(number=Decimal('10.00'), currency='USD', "
        "date=datetime.date(2000, 1, 1), label=None)"
    )
    assert (
        repr(cost_label) == "Cost(number=Decimal('10.00'), currency='USD', "
        "date=datetime.date(2000, 1, 1), label='label')"
    )


def test_equals() -> None:
    assert Amount(Decimal("10.00"), "USD") == Amount(Decimal(10), "USD")
    header = EntryMeta(
        {"filename": "<string>", "lineno": 0},
    )
    assert Balance(
        header,
        date(2022, 12, 12),
        "Assets:Cash",
        Amount(Decimal("10.00"), "USD"),
        None,
        tags={"asdf"},
    ) == Balance(
        header,
        date(2022, 12, 12),
        "Assets:Cash",
        Amount(Decimal(10), "USD"),
        None,
        tags={"asdf"},
    )


def test_custo_value() -> None:
    value = CustomValue("a string", "<AccountDummy>")
    assert value.value == "a string"
    assert value.dtype == "<AccountDummy>"
    value = CustomValue("a string", str)
    assert value.value == "a string"
    assert value.dtype == str
    value = CustomValue(True, bool)  # noqa: FBT003
    assert value.value is True
    assert value.dtype == bool
    value = CustomValue(Decimal("1.0"), Decimal)
    assert value.value == Decimal("1.0")
    assert value.dtype == Decimal


def test_balance() -> None:
    header = EntryMeta({"filename": "<string>", "lineno": 0})
    balance = Balance(
        header,
        date(2022, 12, 12),
        "Assets:Cash",
        Amount(Decimal("10.00"), "USD"),
        None,
        {"asdf"},
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

    assert (
        balance._replace(
            meta={"filename": "<string>", "lineno": 0, "key": "value"}
        ).meta["key"]
        == "value"
    )


HEADER = EntryMeta({"filename": "<string>", "lineno": 0})
DATE = date(2022, 12, 12)
TAGS = {"a-tag"}
LINKS = {"a-link"}


@pytest.mark.parametrize(
    "entry",
    [
        Balance(
            HEADER,
            DATE,
            "A:C",
            Amount(Decimal(1), "USD"),
            None,
            TAGS,
            LINKS,
        ),
        Close(
            HEADER,
            DATE,
            "A:C",
            TAGS,
            LINKS,
        ),
        Commodity(
            HEADER,
            DATE,
            "USD",
            TAGS,
            LINKS,
        ),
        Custom(
            HEADER,
            DATE,
            "custom-type",
            [],
            TAGS,
            LINKS,
        ),
        Document(
            HEADER,
            DATE,
            "Assets:Cash",
            "/path/to/file",
            TAGS,
            LINKS,
        ),
        Event(
            HEADER,
            DATE,
            "event-type",
            "event-name",
            TAGS,
            LINKS,
        ),
        Note(
            HEADER,
            DATE,
            "A:C",
            "account note",
            TAGS,
            LINKS,
        ),
        Open(
            HEADER,
            DATE,
            "A:C",
            ["USD"],
            None,
            TAGS,
            LINKS,
        ),
        Pad(
            HEADER,
            DATE,
            "A:C",
            "A:Source",
            TAGS,
            LINKS,
        ),
        Price(
            HEADER,
            DATE,
            "A:C",
            Amount(Decimal(1), "USD"),
            TAGS,
            LINKS,
        ),
        Query(
            HEADER,
            DATE,
            "name",
            "query",
            TAGS,
            LINKS,
        ),
        Transaction(
            HEADER,
            DATE,
            "*",
            "payee",
            "narration",
            [],
            TAGS,
            LINKS,
        ),
    ],
)
def test_entry_types(entry: Directive) -> None:
    assert hash(entry)
    assert entry == entry._replace(tags={"a-tag"})
    assert entry != entry._replace(tags={"a-different-tag"})
    assert entry.date == date(2022, 12, 12)
    assert entry.links == {"a-link"}
    assert entry.links == {"a-link"}
    assert entry._replace(date=date(2022, 12, 13)).date == date(2022, 12, 13)
    assert entry._replace(tags={"another-tag"}).tags == {"another-tag"}
    assert entry._replace(links={"another-link"}).links == {"another-link"}
    assert entry._replace(meta=entry.meta) == entry

    assert isinstance(entry.meta, EntryMeta)
    assert isinstance(entry.meta, Mapping)
    converted_entry = entry._convert()  # noqa: SLF001
    assert isinstance(converted_entry, data.ALL_DIRECTIVES)
    assert isinstance(converted_entry.meta, dict)
    assert converted_entry.meta == {"filename": "<string>", "lineno": 0}
    assert beancount_to_uromyces(converted_entry)

    with pytest.raises(TypeError, match="takes 0 positional arguments"):
        assert entry._replace("")  # type: ignore[arg-type,misc]


def test_custom_value(load_doc: Ledger) -> None:
    """
    2010-11-11 open Assets:Cash
    2010-11-12 custom "account-name" Assets:Cash
    2010-11-12 custom "multiple-values" "stringy" 2.00 FALSE 2012-10-11
    """
    assert not load_doc.errors
    assert len(load_doc.entries) == 3
    _open, custom_account, custom_multiple = load_doc.entries
    assert isinstance(custom_account, Custom)
    assert custom_account.type == "account-name"
    account_custom_value = custom_account.values[0]
    assert account_custom_value.value == "Assets:Cash"
    assert account_custom_value.dtype == "<AccountDummy>"

    assert isinstance(custom_multiple, Custom)
    (
        string_custom_value,
        decimal_custom_value,
        bool_custom_value,
        date_custom_value,
    ) = custom_multiple.values
    assert string_custom_value.value == "stringy"
    assert string_custom_value.dtype == str
    assert decimal_custom_value.value == Decimal("2.00")
    assert decimal_custom_value.dtype == Decimal
    assert bool_custom_value.value is False
    assert bool_custom_value.dtype == bool
    assert date_custom_value.value == date(2012, 10, 11)
    assert date_custom_value.dtype == date


def test_document() -> None:
    meta = EntryMeta({"filename": "<string>", "lineno": 0})
    day = date(2022, 12, 12)
    document = Document(meta, day, "Assets:Cash", "/path/to/file")
    assert document == Document(
        meta,
        date(2022, 12, 12),
        account="Assets:Cash",
        filename="/path/to/file",
    )
    assert document.filename == "/path/to/file"
    assert document._replace(filename="/other/path").filename == "/other/path"
    assert document._replace(tags={"newtag"}).tags == {"newtag"}


def test_transaction() -> None:
    meta = EntryMeta({"filename": "<string>", "lineno": 0})
    day = date(2022, 12, 12)
    transaction = Transaction(meta, day, "*", "payee", "narration", [])
    assert transaction.flag == "*"
    assert transaction.payee == "payee"
    assert transaction.narration == "narration"
    assert transaction.postings == []

    units = Amount(Decimal("10.00"), "USD")
    posting = Posting("Assets:A1", units, None, None, None, None)
    postings = [posting]
    assert posting == Posting("Assets:A1", units)
    assert posting == Posting(
        "Assets:A1",
        amount.Amount(Decimal("10.00"), "USD"),  # type: ignore[arg-type]
    )
    t = transaction._replace(postings=postings)
    assert t.postings == postings
