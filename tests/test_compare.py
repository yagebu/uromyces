"""Run some checks for uromyces."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from fava.core.tree import Tree

import uromyces
from uromyces._compat import clean_meta
from uromyces._compat import load_beancount
from uromyces._convert import uromyces_to_beancount

if TYPE_CHECKING:
    from pathlib import Path


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

    entries_bc = load_beancount(filename)
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

    for bc, uro in zip(entries_bc, entries_uro):
        clean_meta(bc.meta)
        postings = getattr(bc, "postings", [])
        postings.sort(key=lambda p: p.meta.get("lineno", 0))
        for posting in postings:
            clean_meta(posting.meta)

        assert uromyces_to_beancount(uro) == bc
