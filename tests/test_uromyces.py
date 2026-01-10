from __future__ import annotations

from collections.abc import Mapping
from typing import NamedTuple
from typing import TYPE_CHECKING

import pytest

from uromyces import load_file
from uromyces import load_string
from uromyces.uromyces import Booking
from uromyces.uromyces import Precisions
from uromyces.uromyces import UromycesOptions

if TYPE_CHECKING:
    from pathlib import Path


class _BeancountStyleError(NamedTuple):
    source: dict[str, str | int] | None
    message: str
    entry: None


def test_load_ledger_invalid_path() -> None:
    with pytest.raises(ValueError, match="is not absolute"):
        load_file("not_a_path")
    with pytest.raises(ValueError, match="is not absolute"):
        load_string("", "not_a_path")
    load_string("", "<string>")
    load_string("")


def test_load_ledger(test_ledgers_dir: Path) -> None:
    ledger = load_file(test_ledgers_dir / "example.beancount")
    assert ledger.entries

    assert repr(ledger.entries[0]).startswith("<Commodity")


def test_load_ledger_options(test_ledgers_dir: Path) -> None:
    ledger = load_file(test_ledgers_dir / "example.beancount")
    assert ledger.entries
    options = ledger.options
    assert isinstance(options, UromycesOptions)

    assert options.title == "Example Beancount file"
    assert options.root_accounts.assets == "Assets"
    assert options.root_accounts.equity == "Equity"
    assert options.root_accounts.expenses == "Expenses"
    assert options.root_accounts.income == "Income"
    assert options.root_accounts.liabilities == "Liabilities"
    assert options.account_current_conversions == "Conversions:Current"
    assert options.account_current_earnings == "Earnings:Current"
    assert options.account_previous_balances == "Opening-Balances"
    assert options.account_previous_conversions == "Conversions:Previous"
    assert options.account_previous_earnings == "Earnings:Previous"
    assert not options.render_commas
    assert options.operating_currency == ["USD"]
    assert options.conversion_currency == "NOTHING"
    assert options.documents == []
    assert not options.insert_pythonpath
    assert options.booking_method == Booking.STRICT

    assert isinstance(options.display_precisions, Mapping)
    usd = options.display_precisions["USD"]
    assert usd.has_sign
    assert usd.common == 2
    assert isinstance(usd, Precisions)


def test_ledger_add_error(test_ledgers_dir: Path) -> None:
    ledger = load_file(test_ledgers_dir / "example.beancount")
    assert ledger.entries

    with pytest.raises(AttributeError):
        ledger.add_error(None)
    ledger.add_error(_BeancountStyleError(None, "asdf", None))
    ledger.add_error(_BeancountStyleError({"filename": 12}, "asdf", None))
    ledger.add_error(
        _BeancountStyleError({"filename": "relative"}, "asdf", None)
    )
    ledger.add_error(
        _BeancountStyleError({"filename": "/absolute"}, "asdf", None)
    )
    assert len(ledger.errors) == 4
