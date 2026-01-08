"""Run some checks for uromyces."""

from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING

import pytest
from fava.core.tree import Tree

import uromyces
from uromyces._compare import clean_metadata
from uromyces._compare import isclose_decimal
from uromyces._compat import load_beancount
from uromyces._convert import uromyces_to_beancount

if TYPE_CHECKING:
    from pathlib import Path


def test_isclose() -> None:
    """Test comparison helpers"""
    assert isclose_decimal(
        Decimal("1.00"), Decimal("1.005"), tolerance=Decimal("0.01")
    )
    assert not isclose_decimal(
        Decimal("1.00"), Decimal("1.015"), tolerance=Decimal("0.01")
    )


@pytest.mark.parametrize(
    "ledger_name",
    [
        "example.beancount",
        "long-example.beancount",
    ],
)
def test_compare(test_ledgers_dir: Path, ledger_name: str) -> None:
    """Run some comparison tests between Beancount and uromyces."""
    filename = str(test_ledgers_dir / ledger_name)

    entries_bc, _errors, _options = load_beancount(filename)
    ledger = uromyces.load_file(filename)
    entries_uro = ledger.entries

    balances_bc = {
        n.name: n.balance.to_strings() for n in Tree(entries_bc).values()
    }
    balances_uro = {
        n.name: n.balance.to_strings() for n in Tree(entries_uro).values()
    }
    for account, balance_bc in balances_bc.items():
        assert balance_bc == balances_uro[account], f"Balance for {account}"

    for bc, uro in zip(entries_bc, entries_uro, strict=True):
        clean_metadata(bc)
        postings = getattr(bc, "postings", None)
        if postings is not None:
            postings.sort(key=lambda p: p.meta.get("lineno", 0))

        assert uromyces_to_beancount(uro) == bc
