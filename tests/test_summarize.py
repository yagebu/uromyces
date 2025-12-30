from __future__ import annotations

from datetime import date
from typing import TYPE_CHECKING

import uromyces

if TYPE_CHECKING:
    from uromyces import Ledger


def test_summarize_clamp_prices_before(load_doc: Ledger) -> None:
    """
    2010-11-12 price USD 12 EUR
    2010-11-13 price USD 13 EUR
    2010-11-14 price USD 14 EUR

    2010-11-12 price USD 22 ASDF
    2010-11-13 price USD 23 ASDF
    2010-11-14 price USD 24 ASDF
    """
    entries = load_doc.entries
    assert len(entries) == 6
    clamped_entries = uromyces.summarize_clamp(
        entries, date(2010, 11, 14), date(2010, 11, 16), load_doc.options
    )
    assert len(clamped_entries) == 4
    clamped_entries = uromyces.summarize_clamp(
        entries, date(2011, 11, 14), date(2011, 11, 16), load_doc.options
    )
    assert clamped_entries == [entries[4], entries[5]]


def test_summarize_clamp_keep_open_entries(load_doc: Ledger) -> None:
    """
    2011-01-01 open Assets:Test1
    2012-01-01 open Assets:Test2
    2013-01-01 open Assets:Test3
    2013-01-01 open Assets:Test4
    """
    entries = load_doc.entries
    assert len(entries) == 4

    # None if clamping before
    clamped_entries = uromyces.summarize_clamp(
        entries, date(1990, 1, 1), date(1991, 1, 1), load_doc.options
    )
    assert not clamped_entries

    # All before and the ones in the interval
    clamped_entries = uromyces.summarize_clamp(
        entries, date(2012, 1, 1), date(2012, 1, 2), load_doc.options
    )
    assert clamped_entries == [entries[0], entries[1]]

    # All if clamping after
    clamped_entries = uromyces.summarize_clamp(
        entries, date(2020, 1, 1), date(2021, 1, 1), load_doc.options
    )
    assert clamped_entries == entries
