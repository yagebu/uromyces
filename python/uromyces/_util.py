from __future__ import annotations

import sys
import time
from contextlib import contextmanager
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Generator
    from logging import Logger
    from pathlib import Path


@contextmanager
def log_timing(
    logger: Logger, message: str
) -> Generator[None]:  # pragma: no cover
    """Log the time the wrapped block took.

    Args:
        logger: The logger to use.
        message: The message to log this with.
    """
    before = time.time()
    try:
        yield
    finally:
        elapsed = time.time() - before
        elapsed_ms = elapsed * 1000
        logger.info("%7.3fms - %s", elapsed_ms, message)


@contextmanager
def insert_sys_path(path: Path | None) -> Generator[None]:
    """Insert a path to sys.path for the wrapped block.

    Args:
        path: The path to insert (can be none to make this a noop).
    """
    sys_path_before = sys.path
    if path is not None:
        sys.path.insert(0, str(path))
    try:
        yield
    finally:
        if path is not None:
            sys.path = sys_path_before
