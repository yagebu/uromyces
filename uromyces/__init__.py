"""Uromyces is project to use Rust to implement parts of Beancount."""

from __future__ import annotations

from fava.beans import abc

from uromyces import uromyces
from uromyces._convert import beancount_entries as convert_entries
from uromyces._convert import convert_options
from uromyces._plugins import run
from uromyces._types import Entry
from uromyces.uromyces import Amount
from uromyces.uromyces import Balance
from uromyces.uromyces import Close
from uromyces.uromyces import Commodity
from uromyces.uromyces import Cost
from uromyces.uromyces import Custom
from uromyces.uromyces import Document
from uromyces.uromyces import EntryHeader
from uromyces.uromyces import Event
from uromyces.uromyces import Ledger
from uromyces.uromyces import Note
from uromyces.uromyces import Open
from uromyces.uromyces import Pad
from uromyces.uromyces import Posting
from uromyces.uromyces import Price
from uromyces.uromyces import Query
from uromyces.uromyces import summarize_clamp
from uromyces.uromyces import Transaction

__all__ = [  # noqa: RUF022
    # Entries
    "Balance",
    "Close",
    "Commodity",
    "Custom",
    "Document",
    "Event",
    "Note",
    "Open",
    "Pad",
    "Price",
    "Query",
    "Transaction",
    # Other classes
    "Entry",
    "EntryHeader",
    "Ledger",
    # Functions
    "convert_entries",
    "convert_options",
    "load_file",
    "summarize_clamp",
]


# Register
abc.Amount.register(Amount)
abc.Cost.register(Cost)
abc.Posting.register(Posting)

# Register entry types
abc.Balance.register(Balance)
abc.Close.register(Close)
abc.Commodity.register(Commodity)
abc.Custom.register(Custom)
abc.Document.register(Document)
abc.Event.register(Event)
abc.Note.register(Note)
abc.Open.register(Open)
abc.Pad.register(Pad)
abc.Price.register(Price)
abc.Query.register(Query)
abc.Transaction.register(Transaction)


def load_file(filename: str) -> Ledger:
    """Load a Beancount file.

    Args:
        filename: The string filename to load.

    Returns:
        The ledger.
    """
    ledger = uromyces.load_file(filename)
    ledger = run(ledger)
    ledger.run_validations()
    return ledger
