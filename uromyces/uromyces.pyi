import datetime
from collections.abc import Sequence
from decimal import Decimal
from enum import Enum
from typing import Any

from fava.beans import abc
from fava.beans import protocols
from fava.beans.abc import Meta
from fava.beans.abc import MetaValue

from uromyces._types import Entry

class UroError:
    filename: str | None
    line: int | None
    message: str

class Booking(Enum):
    STRICT = "STRICT"
    NONE = "NONE"
    AVERAGE = "AVERAGE"
    FIFO = "FIFO"
    LIFO = "LIFO"
    HIFO = "HIFO"

class _Directive:
    meta: Meta
    date: datetime.date
    links: frozenset[str]
    tags: frozenset[str]

class Amount:
    number: Decimal
    currency: str

    def __new__(
        cls: Any,
        number: Decimal,
        currency: str,
    ) -> Amount: ...

class Cost:
    number: Decimal
    currency: str
    date: datetime.date | None
    label: str | None

    def __new__(
        cls: Any,
        number: Decimal,
        currency: str,
        date: datetime.date | None,
        label: str | None,
    ) -> Cost: ...

class CustomValue:
    value: MetaValue
    dtype: Any

class EntryHeader:
    date: datetime.date
    filename: str
    tags: frozenset[str]
    links: frozenset[str]

    def __new__(
        cls: Any,
        meta: Meta,
        date: datetime.date,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> EntryHeader: ...

class Balance(_Directive, abc.Balance):
    account: str
    amount: Amount
    tolerance: Decimal | None
    diff_amount: None

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
        amount: protocols.Amount,
        tolerance: Decimal | None,
    ) -> Balance: ...
    def _replace(
        self: Balance,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        amount: protocols.Amount | None = None,
    ) -> Balance: ...

class Close(_Directive, abc.Close):
    account: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
    ) -> Close: ...
    def _replace(
        self: Close,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
    ) -> Close: ...

class Commodity(_Directive, abc.Commodity):
    currency: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        currency: str,
    ) -> Commodity: ...
    def _replace(
        self: Commodity,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        currency: str | None = None,
    ) -> Commodity: ...

class Custom(_Directive, abc.Custom):
    type: str
    values: list[CustomValue]

    def __new__(
        cls: Any,
        header: EntryHeader,
        type: str,  # noqa: A002
        values: list[CustomValue],
    ) -> Custom: ...
    def _replace(
        self: Custom,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        type: str | None = None,  # noqa: A002
        value: list[CustomValue] | None = None,
    ) -> Custom: ...

class Document(_Directive, abc.Document):
    account: str
    filename: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
        filename: str,
    ) -> Document: ...
    def _replace(
        self: Document,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        filename: str | None = None,
    ) -> Document: ...

class Event(_Directive, abc.Event):
    account: str
    type: str
    description: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        type: str,  # noqa: A002
        description: str,
    ) -> Event: ...
    def _replace(
        self: Event,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        type: str | None = None,  # noqa: A002
        description: str | None = None,
    ) -> Event: ...

class Note(_Directive, abc.Note):
    account: str
    comment: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
        comment: str,
    ) -> Note: ...
    def _replace(
        self: Note,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        comment: str | None = None,
    ) -> Note: ...

class Open(_Directive, abc.Open):
    account: str
    currencies: list[str]
    booking: Booking | None  # type: ignore[assignment]

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
        currencies: list[str] | None,
        booking: Booking | None,
    ) -> Open: ...
    def _replace(
        self: Open,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        currencies: list[str] | None = None,
        booking: Booking | None = None,
    ) -> Open: ...

class Pad(_Directive, abc.Pad):
    account: str
    source_account: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        account: str,
        source_account: str,
    ) -> Pad: ...
    def _replace(
        self: Pad,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        source_account: str | None = None,
    ) -> Pad: ...

class Price(_Directive, abc.Price):
    currency: str
    amount: protocols.Amount

    def __new__(
        cls: Any,
        header: EntryHeader,
        currency: str,
        amount: Amount,
    ) -> Price: ...
    def _replace(
        self: Price,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        currency: str | None = None,
        amount: Amount | None = None,
    ) -> Price: ...

class Query(_Directive, abc.Query):
    account: str
    name: str
    query_string: str

    def __new__(
        cls: Any,
        header: EntryHeader,
        name: str,
        query_string: str,
    ) -> Query: ...
    def _replace(
        self: Query,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        name: str | None = None,
        query_string: str | None = None,
    ) -> Query: ...

class Posting(abc.Posting):
    account: str
    units: protocols.Amount
    cost: protocols.Cost | None
    price: protocols.Amount | None
    flag: str | None
    meta: Meta | None

    def __new__(
        cls: Any,
        account: str,
        units: Amount,
        cost: Cost | None = None,
        price: Amount | None = None,
        flag: str | None = None,
        meta: Meta | None = None,
    ) -> Posting: ...

class Transaction(_Directive, abc.Transaction):
    flag: str
    payee: str
    narration: str
    postings: list[Posting]

    def __new__(
        cls: Any,
        header: EntryHeader,
        flag: str,
        payee: str,
        narration: str,
        postings: list[Posting],
    ) -> Transaction: ...
    def _replace(
        self: Transaction,
        *,
        date: datetime.date | None = None,
        meta: Meta | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        flag: str | None = None,
        payee: str | None = None,
        narration: str | None = None,
        postings: list[Posting] | None = None,
    ) -> Transaction: ...

class _RootAccounts:
    assets: str
    liabilities: str
    equity: str
    income: str
    expenses: str

class BeancountOptions:
    title: str
    root_accounts: _RootAccounts
    account_current_conversions: str
    account_current_earnings: str
    account_previous_balances: str
    account_previous_conversions: str
    account_previous_earnings: str
    insert_pythonpath: bool

class Plugin:
    name: str
    config: str | None

class Ledger:
    filename: str
    entries: list[Entry]
    errors: list[UroError]
    includes: list[str]
    options: BeancountOptions
    plugins: list[Plugin]

    def replace_entries(self: Ledger, entries: list[Entry]) -> None: ...
    def add_error(self: Ledger, error: Any) -> None: ...
    def run_validations(self: Ledger) -> None: ...
    def run_plugin(self: Ledger, name: str) -> bool: ...

def load_file(filename: str) -> Ledger: ...
def summarize_clamp(
    entries: Sequence[Entry],
    begin_date: datetime.date,
    end_date: datetime.date,
    options: BeancountOptions,
) -> list[Entry]: ...
