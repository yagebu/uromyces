from __future__ import annotations

from typing import TYPE_CHECKING

from uromyces.uromyces import Balance
from uromyces.uromyces import Close
from uromyces.uromyces import Commodity
from uromyces.uromyces import Custom
from uromyces.uromyces import Document
from uromyces.uromyces import Event
from uromyces.uromyces import Note
from uromyces.uromyces import Open
from uromyces.uromyces import Pad
from uromyces.uromyces import Price
from uromyces.uromyces import Query
from uromyces.uromyces import Transaction

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
