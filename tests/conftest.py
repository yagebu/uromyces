"""Test fixtures."""

from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture(scope="session")
def test_ledgers_dir() -> Path:
    """Path to the test ledgers."""
    return Path(__file__).parent / "ledgers"
