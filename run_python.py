#!/usr/bin/env python
"""Run some checks for uromyces."""
import logging
from abc import ABC
from os import environ
from typing import Any
from decimal import Decimal

import uromyces
from beancount.core import data  # type: ignore
from beancount.loader import _parse_recursive  # type: ignore
from beancount.loader import run_transformations  # type: ignore
from beancount.parser import booking  # type: ignore
from beancount.utils.misc_utils import log_time  # type: ignore


class TransactionABC(ABC):
    """Abstract base class for a Beancount transaction."""


TransactionABC.register(data.Transaction)
TransactionABC.register(uromyces.Transaction)


def load_bc(filename: str):
    """Load the given file using Beancount, without running any plugins."""
    entries, _parse_errors, options_map = _parse_recursive([(filename, True)], None)
    entries.sort(key=data.entry_sortkey)
    entries, _balance_errors = booking.book(entries, options_map)
    entries, _errs = run_transformations(entries, _parse_errors, options_map, None)
    return entries


def _clean_entry_meta(meta: dict[str, Any]):
    meta.pop("__tolerances__", None)
    meta.pop("__implicit_prices__", None)


def _clean_posting_meta(meta: dict[str, Any]):
    meta.pop("__automatic__", None)


def compare_elements(left, right) -> bool:
    """Compare Beancount data types to uromyces data types."""
    if not left and not right:
        return True  # both falsy
    if isinstance(left, data.Booking):
        return left.name in repr(right)
    if isinstance(left, Decimal):
        return left == right
    if isinstance(left, data.Posting):
        if left.account != right.account:
            print("Posting account mismatch: ", left.account, right.account)
            return False
        if left.meta != right.meta:
            print(left.meta, right.meta)
            return False
        if not compare_elements(left.units, right.units):
            return False
        # print(left, right)
        return True  # TODO
    if isinstance(left, data.Amount):
        return True  # TODO
    if isinstance(left, list) and all(isinstance(p, data.Posting) for p in left):
        return all(
            compare_elements(p_left, p_right)
            for p_left, p_right in zip(
                sorted(left, key=lambda p: p.meta["lineno"]), right
            )
        )
    return not left != right


def compare(filename: str, rust_ledger: uromyces.Ledger) -> None:
    entries_bc_all = load_bc(filename)
    entries_bc = [e for e in entries_bc_all if e.meta["filename"] == filename]
    entries_uro_all = rust_ledger.entries
    entries_uro = [e for e in entries_uro_all if e.meta["filename"] == filename]
    for bc, uro in zip(entries_bc, entries_uro):
        entry_type = bc.__class__.__name__

        _clean_entry_meta(bc.meta)
        for posting in getattr(bc, "postings", []):
            _clean_posting_meta(posting.meta)

        for attr, left in bc._asdict().items():
            right = getattr(uro, attr, None)
            if not compare_elements(left, right):
                if entry_type == "Custom":
                    continue  # ignore mismatches of custom values for now
                print("--------------------")
                print(f"Mismatch in {entry_type}.{attr}:")
                print(left)
                print()
                print(right)
                exit()


logging.getLogger().setLevel(logging.INFO)


def main():
    """Run some comparison tests between Beancount and uromyces."""
    path = environ.get("BEANCOUNT_FILE")

    with log_time("loading file", logging.info):
        ledger = uromyces.load_file(path)

    with log_time("compare", logging.info):
        compare(path, ledger)


if __name__ == "__main__":
    main()
