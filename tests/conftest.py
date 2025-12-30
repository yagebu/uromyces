"""Test fixtures."""

from __future__ import annotations

from pathlib import Path
from textwrap import dedent
from typing import TYPE_CHECKING

import pytest

from uromyces import load_string

if TYPE_CHECKING:
    from uromyces import Ledger


@pytest.fixture(scope="session")
def test_ledgers_dir() -> Path:
    """Path to the test ledgers."""
    return Path(__file__).parent / "ledgers"


@pytest.fixture
def load_doc(request: pytest.FixtureRequest) -> Ledger:
    """Load the docstring as a Beancount file."""
    contents = dedent(request.function.__doc__)
    return load_string(contents, str(request.path))
