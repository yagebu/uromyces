from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING

from beancount.core import data
from beancount.core import position

if TYPE_CHECKING:
    from collections.abc import Sequence

    from beancount.core import amount
    from fava.beans.protocols import Amount
    from fava.beans.protocols import Cost


TOLERANCE = Decimal("0.000000000000000000001")
ZERO = Decimal()


def isclose_decimal(self: Decimal, other: Decimal, tolerance: Decimal) -> bool:
    return abs(self - other) < tolerance


def isclose_amount(
    self: amount.Amount | Amount | None,
    other: amount.Amount | Amount | None,
    tolerance: Decimal,
) -> bool:
    if self is None or other is None:
        return self == other
    return self.currency == other.currency and isclose_decimal(
        self.number or ZERO, other.number or ZERO, tolerance
    )


def isclose_cost(
    self: position.Cost | position.CostSpec | Cost | None,
    other: position.Cost | position.CostSpec | Cost | None,
    tolerance: Decimal,
) -> bool:
    if (
        self is None
        or isinstance(self, position.CostSpec)
        or other is None
        or isinstance(other, position.CostSpec)
    ):
        return self == other
    return (
        self.currency == other.currency
        and isclose_decimal(
            self.number or ZERO, other.number or ZERO, tolerance
        )
        and self.date == other.date
        and self.label == other.label
    )


def clean_metadata(entry: data.Directive) -> None:
    """Remove some metadata from entry and postings.

    These are not set by uromyces (yet?), so remove them.
    """
    entry.meta.pop("__tolerances__", None)
    for posting in getattr(entry, "postings", []):
        posting.meta.pop("__automatic__", None)


def compare_postings(
    self: Sequence[data.Posting],
    other: Sequence[data.Posting],
    tolerance: Decimal = TOLERANCE,
) -> bool:
    return len(self) == len(other) and all(
        s.account == o.account
        and isclose_amount(s.units, o.units, tolerance)
        and isclose_cost(s.cost, o.cost, tolerance)
        and isclose_amount(s.price, o.price, tolerance)
        and s.flag == o.flag
        and s.meta == o.meta
        for s, o in zip(self, other, strict=True)
    )


def compare_entries(
    self: data.Directive, other: data.Directive, tolerance: Decimal = TOLERANCE
) -> bool:
    """Compare entries with tolerance for Decimals."""
    if isinstance(self, data.Price) and isinstance(other, data.Price):
        return (
            self.meta == other.meta
            and self.date == other.date
            and self.currency == other.currency
            and isclose_amount(self.amount, other.amount, tolerance)
        )
    if isinstance(self, data.Transaction) and isinstance(
        other, data.Transaction
    ):
        return (
            self.meta == other.meta
            and self.date == other.date
            and self.flag == other.flag
            and self.payee == other.payee
            and self.narration == other.narration
            and self.tags == other.tags
            and self.links == other.links
            and compare_postings(self.postings, other.postings, tolerance)
        )
    return self == other
