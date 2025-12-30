from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from fava.helpers import BeancountError

from uromyces import load_file
from uromyces import load_string

if TYPE_CHECKING:
    from pathlib import Path


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


def test_ledger_add_error(test_ledgers_dir: Path) -> None:
    ledger = load_file(test_ledgers_dir / "example.beancount")
    assert ledger.entries

    with pytest.raises(AttributeError):
        ledger.add_error(None)
    ledger.add_error(BeancountError(None, "asdf", None))
    ledger.add_error(BeancountError({"filename": 12}, "asdf", None))
    ledger.add_error(BeancountError({"filename": "relative"}, "asdf", None))
    ledger.add_error(BeancountError({"filename": "/absolute"}, "asdf", None))
    assert len(ledger.errors) == 4
