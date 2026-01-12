from __future__ import annotations

from typing import TYPE_CHECKING

from beancount import loader

if TYPE_CHECKING:
    from beancount.core import data
    from fava.beans.types import BeancountOptions


def load_beancount(
    filename: str,
) -> tuple[list[data.Directive], list[data.BeancountError], BeancountOptions]:
    """Load the given file using Beancount."""
    entries, errors, options_map = loader._load(  # noqa: SLF001
        [(filename, True)],
        None,
        None,
        None,
    )
    return entries, errors, options_map
