"""Helpers to convert from uromyces data types to Beancount."""

from __future__ import annotations

import copy
from functools import singledispatch
from typing import TYPE_CHECKING

from beancount.core import data
from beancount.parser.options import OPTIONS_DEFAULTS

from uromyces.uromyces import Balance
from uromyces.uromyces import Booking
from uromyces.uromyces import Close
from uromyces.uromyces import Commodity
from uromyces.uromyces import Custom
from uromyces.uromyces import CustomValue
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
    from collections.abc import Sequence

    from fava.beans.types import BeancountOptions

    from uromyces import Directive
    from uromyces import Ledger


def beancount_entries(entries: Sequence[Directive]) -> list[data.Directive]:
    """Convert entries of the ledger to Beancount entries."""
    return [entry._convert() for entry in entries]  # noqa: SLF001


def uromyces_entries(
    entries: Sequence[Directive | data.Directive],
) -> list[Directive]:
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
    return opts  # type: ignore[return-value]


@singledispatch
def beancount_to_uromyces(entry: Directive | data.Directive) -> Directive:
    """Convert a Beancount Entry to a uromyces entry."""
    return entry  # type: ignore[return-value]


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
        [CustomValue(v.value, v.dtype) for v in entry.values],
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


@beancount_to_uromyces.register(data.Pad)
def _(entry: data.Pad) -> Pad:
    return Pad(
        EntryHeader(entry.meta, entry.date),
        entry.account,
        entry.source_account,
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
