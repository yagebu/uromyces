"""Helpers to convert from uromyces data types to Beancount."""

from __future__ import annotations

import copy
from functools import singledispatch
from typing import TYPE_CHECKING

from beancount.core import amount
from beancount.core import data
from beancount.core import position
from beancount.parser.grammar import ValueType
from beancount.parser.options import OPTIONS_DEFAULTS

from uromyces.uromyces import Balance
from uromyces.uromyces import Booking
from uromyces.uromyces import Close
from uromyces.uromyces import Commodity
from uromyces.uromyces import Custom
from uromyces.uromyces import Document
from uromyces.uromyces import EntryHeader
from uromyces.uromyces import Event
from uromyces.uromyces import Note
from uromyces.uromyces import Open
from uromyces.uromyces import Pad
from uromyces.uromyces import Posting
from uromyces.uromyces import Price
from uromyces.uromyces import Query
from uromyces.uromyces import Transaction

if TYPE_CHECKING:
    from typing import Any

    from fava.beans import abc
    from fava.beans.types import BeancountOptions

    from uromyces import Entry
    from uromyces import Ledger


def beancount_entries(entries: list[Entry]) -> list[Any]:
    """Convert entries of the ledger to Beancount entries."""
    return list(map(uromyces_to_beancount, entries))


def uromyces_entries(entries: list[Any]) -> list[Entry]:
    """Convert Beancount entries to uromyces."""
    return list(map(beancount_to_uromyces, entries))


def convert_options(ledger: Ledger) -> BeancountOptions:
    """Convert the options for the given Ledger to Beancount's option dict."""
    opts = copy.copy(OPTIONS_DEFAULTS)
    opts["include"] = ledger.includes
    opts["filename"] = ledger.filename
    for option_name in opts:
        if hasattr(ledger.options, option_name):
            opts[option_name] = getattr(
                ledger.options,
                option_name,
            )
    root_accounts = ledger.options.root_accounts
    opts["name_assets"] = root_accounts.assets
    opts["name_liabilities"] = root_accounts.liabilities
    opts["name_equity"] = root_accounts.equity
    opts["name_income"] = root_accounts.income
    opts["name_expenses"] = root_accounts.expenses
    opts["_uro_options"] = ledger.options
    return opts  # type: ignore[return-value]


@singledispatch
def uromyces_to_beancount(_: Entry) -> Any:
    """Convert a uromyces Entry to a Beancount entry."""
    raise NotImplementedError


@uromyces_to_beancount.register(Balance)
def _(entry: Balance) -> data.Balance:
    return data.Balance(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
        entry.amount,  # type: ignore[arg-type]
        entry.tolerance,
        None,
    )


@uromyces_to_beancount.register(Commodity)
def _(entry: Commodity) -> data.Commodity:
    return data.Commodity(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.currency,
    )


@uromyces_to_beancount.register(Close)
def _(entry: Close) -> data.Close:
    return data.Close(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
    )


@uromyces_to_beancount.register(Custom)
def _(entry: Custom) -> data.Custom:
    return data.Custom(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.type,
        [ValueType(v.value, v.dtype) for v in entry.values],
    )


@uromyces_to_beancount.register(Document)
def _(entry: Document) -> data.Document:
    return data.Document(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
        entry.filename,
        entry.tags,
        entry.links,
    )


@uromyces_to_beancount.register(Event)
def _(entry: Event) -> data.Event:
    return data.Event(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.type,
        entry.description,
    )


@uromyces_to_beancount.register(Note)
def _(entry: Note) -> data.Note:
    return data.Note(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
        entry.comment,
        entry.tags,
        entry.links,
    )


@uromyces_to_beancount.register(Open)
def _(entry: Open) -> data.Open:
    return data.Open(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
        entry.currencies or None,  # type: ignore[arg-type]
        None
        if entry.booking is None
        else getattr(data.Booking, entry.booking.value),
    )


@uromyces_to_beancount.register(Pad)
def _(entry: Pad) -> data.Pad:
    return data.Pad(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.account,
        entry.source_account,
    )


@uromyces_to_beancount.register(Price)
def _(entry: Price) -> data.Price:
    return data.Price(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.currency,
        entry.amount,  # type: ignore[arg-type]
    )


@uromyces_to_beancount.register(Query)
def _(entry: Query) -> data.Query:
    return data.Query(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.name,
        entry.query_string,
    )


def _posting_to_beancount(pos: Posting) -> data.Posting:
    units = pos.units
    cost = pos.cost
    price = pos.price
    return data.Posting(
        pos.account,
        amount.Amount(units.number, units.currency),
        None
        if cost is None
        else position.Cost(
            cost.number,
            cost.currency,
            cost.date,
            cost.label,
        ),
        None if price is None else amount.Amount(price.number, price.currency),
        pos.flag,
        pos.meta if pos.meta else None,  # type: ignore[arg-type]
    )


@uromyces_to_beancount.register(Transaction)
def _(entry: Transaction) -> data.Transaction:
    postings = [_posting_to_beancount(p) for p in entry.postings]
    return data.Transaction(
        entry.meta,  # type: ignore[arg-type]
        entry.date,
        entry.flag,
        entry.payee,
        entry.narration,
        entry.tags,
        entry.links,
        postings,
    )


_UroEntryTypes = (
    Balance,
    Close,
    Commodity,
    Custom,
    Document,
    Event,
    Note,
    Open,
    Pad,
    Price,
    Query,
    Transaction,
)


@singledispatch
def beancount_to_uromyces(entry: abc.Directive | data.Directive) -> Entry:
    """Convert a Beancount Entry to a uromyces entry."""
    if isinstance(entry, _UroEntryTypes):
        return entry
    raise NotImplementedError


@beancount_to_uromyces.register(data.Balance)
def _(entry: data.Balance) -> Balance:
    return Balance(
        EntryHeader(entry.meta, entry.date),
        entry.account,
        entry.amount,  # type: ignore[arg-type]
        entry.tolerance,
    )


@beancount_to_uromyces.register(data.Commodity)
def _(entry: data.Commodity) -> Commodity:
    return Commodity(
        EntryHeader(entry.meta, entry.date),
        entry.currency,
    )


@beancount_to_uromyces.register(data.Close)
def _(entry: data.Close) -> Close:
    return Close(
        EntryHeader(entry.meta, entry.date),
        entry.account,
    )


@beancount_to_uromyces.register(data.Custom)
def _(entry: data.Custom) -> Custom:
    return Custom(
        EntryHeader(entry.meta, entry.date),
        entry.type,
        entry.values,
    )


@beancount_to_uromyces.register(data.Document)
def _(entry: data.Document) -> Document:
    return Document(
        EntryHeader(entry.meta, entry.date, entry.tags, entry.links),
        entry.account,
        entry.filename,
    )


@beancount_to_uromyces.register(data.Event)
def _(entry: data.Event) -> Event:
    return Event(
        EntryHeader(entry.meta, entry.date),
        entry.type,
        entry.description,
    )


@beancount_to_uromyces.register(data.Open)
def _(entry: data.Open) -> Open:
    return Open(
        EntryHeader(entry.meta, entry.date),
        entry.account,
        entry.currencies or [],
        None
        if entry.booking is None
        else getattr(Booking, entry.booking.value),
    )


@beancount_to_uromyces.register(data.Note)
def _(entry: data.Note) -> Note:
    return Note(
        EntryHeader(entry.meta, entry.date),
        entry.account,
        entry.comment,
    )


@beancount_to_uromyces.register(data.Price)
def _(entry: data.Price) -> Price:
    return Price(
        EntryHeader(entry.meta, entry.date),
        entry.currency,
        entry.amount,  # type: ignore[arg-type]
    )


@beancount_to_uromyces.register(data.Query)
def _(entry: data.Query) -> Query:
    return Query(
        EntryHeader(entry.meta, entry.date),
        entry.name,
        entry.query_string,
    )


@beancount_to_uromyces.register(data.Transaction)
def _(entry: data.Transaction) -> Transaction:
    return Transaction(
        EntryHeader(entry.meta, entry.date, entry.tags, entry.links),
        entry.flag or "*",
        entry.payee or "",
        entry.narration,  # type: ignore[arg-type]
        [
            Posting(
                p.account,
                p.units,  # type: ignore[arg-type]
                p.cost,  # type: ignore[arg-type]
                p.price,  # type: ignore[arg-type]
                p.flag,
                p.meta,
            )
            for p in entry.postings
        ],
    )
