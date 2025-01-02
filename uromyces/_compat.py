from __future__ import annotations

from typing import Any

from beancount import loader


def clean_meta(meta: dict[str, Any]) -> None:
    """Clean entry or posting metadata."""
    meta.pop("__tolerances__", None)
    meta.pop("__automatic__", None)


def load_beancount(filename: str) -> Any:
    """Load the given file using Beancount."""
    entries, _parse_errors, options_map = loader._load(  # noqa: SLF001
        [(filename, True)],
        None,
        None,
        None,
    )
    assert not _parse_errors  # noqa: S101
    return entries
