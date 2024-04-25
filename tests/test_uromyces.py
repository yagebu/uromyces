from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from fava.helpers import BeancountError

import uromyces

if TYPE_CHECKING:
    from pathlib import Path


def test_load_ledger(test_ledgers_dir: Path) -> None:
    ledger = uromyces.load_file(str(test_ledgers_dir / "example.beancount"))
    assert ledger.entries


def test_ledger_add_error(test_ledgers_dir: Path) -> None:
    ledger = uromyces.load_file(str(test_ledgers_dir / "example.beancount"))
    assert ledger.entries

    with pytest.raises(AttributeError):
        ledger.add_error(None)
    ledger.add_error(BeancountError(None, "asdf", None))
    ledger.add_error(BeancountError({"filename": 12}, "asdf", None))
    ledger.add_error(BeancountError({"filename": "relative"}, "asdf", None))
    ledger.add_error(BeancountError({"filename": "/absolute"}, "asdf", None))
    assert len(ledger.errors) == 4  # noqa: PLR2004
