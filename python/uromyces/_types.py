from __future__ import annotations

from typing import TYPE_CHECKING

from uromyces._uromyces import Balance
from uromyces._uromyces import Close
from uromyces._uromyces import Commodity
from uromyces._uromyces import Custom
from uromyces._uromyces import Document
from uromyces._uromyces import Event
from uromyces._uromyces import Note
from uromyces._uromyces import Open
from uromyces._uromyces import Pad
from uromyces._uromyces import Price
from uromyces._uromyces import Query
from uromyces._uromyces import Transaction

if TYPE_CHECKING:
    from typing import TypeAlias

Directive: TypeAlias = (
    Balance
    | Close
    | Commodity
    | Custom
    | Document
    | Event
    | Note
    | Open
    | Pad
    | Price
    | Query
    | Transaction
)
