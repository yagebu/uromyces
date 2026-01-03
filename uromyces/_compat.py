from __future__ import annotations

from typing import TYPE_CHECKING

from beancount import loader

if TYPE_CHECKING:
    from beancount.core import data


def load_beancount(
    filename: str,
) -> tuple[list[data.Directive], list[data.BeancountError]]:
    """Load the given file using Beancount."""
    entries, errors, _options_map = loader._load(  # noqa: SLF001
        [(filename, True)],
        None,
        None,
        None,
    )
    return entries, errors
