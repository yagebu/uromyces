import datetime
from collections.abc import ItemsView
from collections.abc import Iterator
from collections.abc import KeysView
from collections.abc import Mapping
from collections.abc import Sequence
from collections.abc import ValuesView
from decimal import Decimal
from enum import Enum
from typing import Any
from typing import final
from typing import Literal
from typing import overload
from typing import TypeAlias

from beancount.core import data
from fava.beans import abc
from fava.beans import protocols
from fava.beans.abc import Meta
from fava.beans.abc import MetaValue

from uromyces._types import Directive

class UroError:
    filename: str | None
    lineno: int | None
    source: Meta
    message: str
    entry: Directive | None

class Booking(Enum):
    STRICT = "STRICT"
    NONE = "NONE"
    AVERAGE = "AVERAGE"
    FIFO = "FIFO"
    LIFO = "LIFO"
    HIFO = "HIFO"

class _Directive:
    meta: EntryMeta
    date: datetime.date
    links: frozenset[str]
    tags: frozenset[str]

    def _convert(self) -> data.Directive: ...

@final
class Amount:
    number: Decimal
    currency: str

    def __new__(
        cls: type[Amount],
        number: Decimal,
        currency: str,
    ) -> Amount: ...

@final
class RawAmount:
    number: Decimal | None
    currency: str | None

    def __new__(
        cls: type[RawAmount],
        number: Decimal | None,
        currency: str | None,
    ) -> RawAmount: ...

@final
class Cost:
    number: Decimal
    currency: str
    date: datetime.date | None
    label: str | None

    def __new__(
        cls: type[Cost],
        number: Decimal,
        currency: str,
        date: datetime.date | None,
        label: str | None,
    ) -> Cost: ...

@final
class CostSpec:
    number_per: Decimal | None
    number_total: Decimal | None
    currency: str | None
    date: datetime.date | None
    label: str | None
    merge: bool

    def __new__(
        cls: type[CostSpec],
        number_per: Decimal | None,
        number_total: Decimal | None,
        currency: str | None,
        date: datetime.date | None,
        label: str | None,
        merge: bool,
    ) -> CostSpec: ...

class CustomValue:
    value: MetaValue
    dtype: type[MetaValue] | Literal["<AccountDummy>"]

    @overload
    def __new__(
        cls: type[CustomValue], value: str, dtype: Literal["<AccountDummy>"]
    ) -> CustomValue: ...
    @overload
    def __new__(
        cls: type[CustomValue], value: int, dtype: type[int]
    ) -> CustomValue: ...
    @overload
    def __new__(
        cls: type[CustomValue], value: Decimal, dtype: type[Decimal]
    ) -> CustomValue: ...
    @overload
    def __new__(
        cls: type[CustomValue],
        value: protocols.Amount,
        dtype: type[protocols.Amount],
    ) -> CustomValue: ...
    @overload
    def __new__(
        cls: type[CustomValue], value: str, dtype: type[str]
    ) -> CustomValue: ...
    @overload
    def __new__(
        cls: type[CustomValue],
        value: datetime.date,
        dtype: type[datetime.date],
    ) -> CustomValue: ...

@final
class EntryMeta(Mapping[str, MetaValue]):
    filename: str
    lineno: int

    def __new__(cls: type[EntryMeta], meta: Meta) -> EntryMeta: ...
    def __contains__(self, key: object) -> bool: ...
    def __getitem__(self, key: str) -> MetaValue: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def items(self) -> ItemsView[str, MetaValue]: ...
    def keys(self) -> KeysView[str]: ...
    def values(self) -> ValuesView[MetaValue]: ...
    def copy(self) -> dict[str, MetaValue]: ...

@final
class PostingMeta(Mapping[str, MetaValue]):
    filename: str | None
    lineno: int | None

    def __new__(cls: type[PostingMeta], meta: Meta) -> PostingMeta: ...
    def __contains__(self, key: object) -> bool: ...
    def __getitem__(self, key: str) -> MetaValue: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def items(self) -> ItemsView[str, MetaValue]: ...
    def keys(self) -> KeysView[str]: ...
    def values(self) -> ValuesView[MetaValue]: ...
    def copy(self) -> dict[str, MetaValue]: ...

@final
class Balance(_Directive, abc.Balance):
    account: str
    amount: Amount
    tolerance: Decimal | None
    diff_amount: None

    def __new__(
        cls: type[Balance],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        amount: protocols.Amount,
        tolerance: Decimal | None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Balance: ...
    def _replace(
        self: Balance,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        amount: protocols.Amount | None = None,
    ) -> Balance: ...

@final
class Close(_Directive, abc.Close):
    account: str

    def __new__(
        cls: type[Close],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Close: ...
    def _replace(
        self: Close,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
    ) -> Close: ...

@final
class Commodity(_Directive, abc.Commodity):
    currency: str

    def __new__(
        cls: type[Commodity],
        meta: EntryMeta | Meta,
        date: datetime.date,
        currency: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Commodity: ...
    def _replace(
        self: Commodity,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        currency: str | None = None,
    ) -> Commodity: ...

# workaround type-checkers confusing 'type' with the function param below:
_CustomType: TypeAlias = type[Custom]
_EventType: TypeAlias = type[Event]

@final
class Custom(_Directive, abc.Custom):
    type: str
    values: list[CustomValue]

    def __new__(
        cls: _CustomType,
        meta: EntryMeta | Meta,
        date: datetime.date,
        type: str,  # noqa: A002
        values: list[CustomValue],
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Custom: ...
    def _replace(
        self: Custom,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        type: str | None = None,  # noqa: A002
        value: list[CustomValue] | None = None,
    ) -> Custom: ...

@final
class Document(_Directive, abc.Document):
    account: str
    filename: str

    def __new__(
        cls: type[Document],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        filename: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Document: ...
    def _replace(
        self: Document,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        filename: str | None = None,
    ) -> Document: ...

@final
class Event(_Directive, abc.Event):
    account: str
    type: str
    description: str

    def __new__(
        cls: _EventType,
        meta: EntryMeta | Meta,
        date: datetime.date,
        type: str,  # noqa: A002
        description: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Event: ...
    def _replace(
        self: Event,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        type: str | None = None,  # noqa: A002
        description: str | None = None,
    ) -> Event: ...

@final
class Note(_Directive, abc.Note):
    account: str
    comment: str

    def __new__(
        cls: type[Note],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        comment: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Note: ...
    def _replace(
        self: Note,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        comment: str | None = None,
    ) -> Note: ...

@final
class Open(_Directive, abc.Open):
    account: str
    currencies: list[str]
    booking: Booking | None  # type: ignore[assignment]

    def __new__(
        cls: type[Open],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        currencies: list[str] | None,
        booking: Booking | None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Open: ...
    def _replace(
        self: Open,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        currencies: list[str] | None = None,
        booking: Booking | None = None,
    ) -> Open: ...

@final
class Pad(_Directive, abc.Pad):
    account: str
    source_account: str

    def __new__(
        cls: type[Pad],
        meta: EntryMeta | Meta,
        date: datetime.date,
        account: str,
        source_account: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Pad: ...
    def _replace(
        self: Pad,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        account: str | None = None,
        source_account: str | None = None,
    ) -> Pad: ...

@final
class Price(_Directive, abc.Price):
    currency: str
    amount: protocols.Amount

    def __new__(
        cls: type[Price],
        meta: EntryMeta | Meta,
        date: datetime.date,
        currency: str,
        amount: Amount,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Price: ...
    def _replace(
        self: Price,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        currency: str | None = None,
        amount: Amount | None = None,
    ) -> Price: ...

@final
class Query(_Directive, abc.Query):
    account: str
    name: str
    query_string: str

    def __new__(
        cls: type[Query],
        meta: EntryMeta | Meta,
        date: datetime.date,
        name: str,
        query_string: str,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Query: ...
    def _replace(
        self: Query,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        name: str | None = None,
        query_string: str | None = None,
    ) -> Query: ...

@final
class Posting(abc.Posting):
    account: str
    units: protocols.Amount
    cost: protocols.Cost | None
    price: protocols.Amount | None
    flag: str | None
    meta: Meta | None

    def __new__(
        cls: type[Posting],
        account: str,
        units: Amount,
        cost: Cost | None = None,
        price: Amount | None = None,
        flag: str | None = None,
        meta: Meta | None = None,
    ) -> Posting: ...

@final
class Transaction(_Directive, abc.Transaction):
    flag: str
    payee: str
    narration: str
    postings: list[Posting]

    def __new__(
        cls: type[Transaction],
        meta: EntryMeta | Meta,
        date: datetime.date,
        flag: str,
        payee: str,
        narration: str,
        postings: list[Posting],
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
    ) -> Transaction: ...
    def _replace(
        self: Transaction,
        *,
        meta: Meta | None = None,
        date: datetime.date | None = None,
        tags: set[str] | frozenset[str] | None = None,
        links: set[str] | frozenset[str] | None = None,
        flag: str | None = None,
        payee: str | None = None,
        narration: str | None = None,
        postings: list[Posting] | None = None,
    ) -> Transaction: ...

@final
class RawPosting:
    account: str
    units: RawAmount
    cost: CostSpec | None
    price: RawAmount | None
    flag: str | None
    meta: Meta | None

@final
class RawTransaction(_Directive):
    flag: str
    payee: str
    narration: str
    postings: list[RawPosting]

class RootAccounts:
    assets: str
    liabilities: str
    equity: str
    income: str
    expenses: str

class Precisions:
    has_sign: bool
    max: int
    common: int

class UromycesOptions:
    title: str
    root_accounts: RootAccounts
    account_current_conversions: str
    account_current_earnings: str
    account_previous_balances: str
    account_previous_conversions: str
    account_previous_earnings: str
    render_commas: bool
    operating_currency: Sequence[str]
    conversion_currency: str
    documents: Sequence[str]
    booking_method: Booking
    insert_pythonpath: bool
    display_precisions: Mapping[str, Precisions]

class Plugin:
    name: str
    config: str | None

class Ledger:
    filename: str
    entries: list[Directive]
    errors: list[UroError]
    includes: list[str]
    options: UromycesOptions
    plugins: list[Plugin]

    def replace_entries(self: Ledger, entries: list[Directive]) -> None: ...
    def add_error(self: Ledger, error: Any) -> None: ...
    def run_validations(self: Ledger) -> None: ...
    def run_plugin(self: Ledger, name: str) -> bool: ...

def load_file(filename: str) -> Ledger: ...
def load_string(string: str, filename: str) -> Ledger: ...
def summarize_clamp(
    entries: Sequence[Directive],
    begin_date: datetime.date,
    end_date: datetime.date,
    options: UromycesOptions,
) -> list[Directive]: ...
