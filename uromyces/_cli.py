"""The command-line interface for uromyces."""

from __future__ import annotations

import logging
import os
import time
from decimal import getcontext
from logging import info
from sys import stderr

import click

from uromyces import load_file
from uromyces._compat import clean_meta
from uromyces._compat import load_beancount
from uromyces._convert import uromyces_to_beancount


class NoFileGivenError(click.UsageError):
    def __init__(self: NoFileGivenError) -> None:
        super().__init__("No file specified")


@click.command()
@click.argument(
    "filenames",
    nargs=-1,
    type=click.Path(exists=True, dir_okay=False, resolve_path=True),
)
@click.option(
    "-c",
    "--compare",
    is_flag=True,
    help="Compare output to Beancount.",
)
@click.option(
    "-v",
    "--verbose",
    is_flag=True,
    help="Verbose output.",
)
def cli(
    *,
    filenames: tuple[str, ...] = (),
    verbose: bool = False,
    compare: bool = False,
) -> None:  # pragma: no cover
    """Run uro for FILENAMES.

    If the `BEANCOUNT_FILE` environment variable is set, it will use the
    files (space-delimited) specified there in addition to FILENAMES.
    """
    logging.basicConfig(level=logging.INFO if verbose else logging.WARNING)
    getcontext().prec = 29

    env_filename = os.environ.get("BEANCOUNT_FILE")
    all_filenames = (
        filenames + tuple(env_filename.split(os.pathsep))
        if env_filename
        else filenames
    )

    if not all_filenames:
        raise NoFileGivenError

    info("Running uro for %s", all_filenames)

    for filename in all_filenames:
        before = time.time()
        info("Load Beancount ledger: %s", filename)

        ledger = load_file(filename)

        info(
            "Loaded Beancount ledger (%s entries; %s errors) in %s",
            len(ledger.entries),
            len(ledger.errors),
            time.time() - before,
        )

        if compare:
            entries_uro = ledger.entries
            entries_bc = load_beancount(filename)
            for bc, uro in zip(entries_bc, entries_uro):
                clean_meta(bc.meta)
                for posting in getattr(bc, "postings", ()):
                    clean_meta(posting.meta)

                converted = uromyces_to_beancount(uro)
                if converted != bc:
                    click.echo(uro)
                    click.echo(bc)
                    click.echo(converted)
                    raise click.UsageError("")  # noqa: EM101

        for error in ledger.errors:
            msg = click.style(error.message, fg="red")
            click.echo(
                f"{error.filename}:{error.line}:{msg}",
                file=stderr,
            )
