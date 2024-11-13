#!/usr/bin/env python3
"""Convert tests from Beancount's booking_full_test.py into expect files."""

from __future__ import annotations

import re
import sys
from inspect import getmembers
from inspect import getsource
from inspect import isclass
from pathlib import Path
from textwrap import dedent

from click import group
from click import secho

BASE_PATH = Path(__file__).parent.parent
BEANCOUNT_PATH = BASE_PATH.parent / "beancount"
BOOKING_TEST_PATH = (
    BASE_PATH / "src" / "booking" / "booking_full_tests_imported"
)


@group()
def cli() -> None:
    """Various utilities."""


IGNORED_TESTS = {
    # These try to reduce 0 HOOL {} which panics right now
    "LIFO_test_ambiguous__LIFO__no_match_against_any_lots",
    "FIFO_test_ambiguous__FIFO__no_match_against_any_lots",
    # This has a missing units number - uromyces currently books a zero posting
    # here
    "STRICT_test_reduce__missing_units_number",
}


def _format_snapshot(title: str, contents: str, expected: str) -> str:
    """Format a Beancount snapshot."""
    header_sep_line = f";{'='*78}\n"
    sep_line = f";{'-'*78}\n"
    expected_escaped = "\n; ".join(expected.split("\n"))
    return (
        f"{header_sep_line}; {title}\n{header_sep_line}"
        f"{contents}"
        f"{sep_line}; {expected_escaped}\n"
    )


@cli.command()
def import_booking_tests() -> None:
    """Import booking tests from beancount.parser.booking_full_test.

    Expects Beancount's repo to live next to uromyces.
    """
    sys.path.insert(0, str(BEANCOUNT_PATH))
    from beancount.parser import (  # type: ignore[import-untyped]
        booking_full_test,
    )

    sys.path.pop(0)

    BOOKING_TEST_PATH.mkdir(exist_ok=True)
    base_cls = booking_full_test._BookingTestBase  # noqa: SLF001

    ignored_test_ids = set()
    imported_test_ids = set()

    skipped_classes = set()

    for cls_name, member in (
        (name, member)
        for name, member in getmembers(booking_full_test)
        if isclass(member) and issubclass(member, base_cls)
    ):
        source = getsource(member)
        match = re.search(r"@unittest.skip", source)
        if match:
            skipped_classes.add(cls_name)
            continue
        secho(f"INFO: tests in {cls_name}", fg="green")

        for method_name, method in (
            (name, member)
            for name, member in getmembers(member)
            if name.startswith("test_")
        ):
            source = getsource(method)
            match = re.search(r"@book_test\(Booking\.(.*)\)", source)
            assert match
            booking_method = match.group(1)

            # identify a test by booking method and method name
            test_id = f"{booking_method}_{method_name}"

            if test_id in IGNORED_TESTS:
                secho(f"IGNORED: skipped test for now: {test_id}", fg="yellow")
                ignored_test_ids.add(test_id)
                continue

            excluded = {"STRICT_WITH_SIZE"}
            if booking_method not in excluded:
                target_path = BOOKING_TEST_PATH / f"{test_id}.beancount"
                contents = dedent(method.__doc__)
                # uro-parser doesn't support txns without postings, add dummy
                contents = re.sub(
                    r"error: \".*\"", r"\g<0>\n  Assets:Dummy", contents
                )
                contents = re.sub(
                    r"(#\w+)\n(?! )", r"\g<1>\n  Assets:Dummy\n", contents
                )

                assert test_id not in imported_test_ids
                imported_test_ids.add(test_id)

                if not target_path.exists():
                    snapshot_contents = _format_snapshot(
                        title=test_id,
                        contents=contents + "\n",
                        expected="EXPECTED",
                    )
                    target_path.write_text(snapshot_contents)
                    secho(f"IMPORTED: {test_id}", fg="green")
                else:
                    secho(f"IGNORED: already exists: {test_id}", fg="green")
            else:
                secho(
                    f"IGNORED: method {booking_method} not yet implemented",
                    fg="yellow",
                )

    secho(
        "INFO: skippd tests in classes marked with `@unittest.skip`:\n"
        f" {skipped_classes}",
        fg="yellow",
    )

    unused_ignores = IGNORED_TESTS - ignored_test_ids
    if unused_ignores:
        secho(f"ignore not used: {unused_ignores}", fg="red")
        sys.exit(1)


if __name__ == "__main__":
    cli()
