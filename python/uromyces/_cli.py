"""The command-line interface for uromyces."""

from __future__ import annotations

import logging
import time
from logging import getLogger
from pprint import pformat

import click

from uromyces import load_file
from uromyces._compare import compare_entries
from uromyces._compat import load_beancount
from uromyces._convert import beancount_entries
from uromyces._convert import convert_options

logger = getLogger(__name__)


class NoFileGivenError(click.UsageError):
    def __init__(self: NoFileGivenError) -> None:
        super().__init__("No file specified")


@click.group()
def cli() -> None: ...


FILENAME_TYPE = click.Path(exists=True, dir_okay=False, resolve_path=True)


@cli.command()
@click.argument("filenames", nargs=-1, type=FILENAME_TYPE)
@click.option("-v", "--verbose", is_flag=True, help="Verbose output.")
def check(*, filenames: tuple[str, ...], verbose: bool) -> None:
    """Run uro for FILENAMES."""
    if not filenames:
        raise NoFileGivenError

    logging.basicConfig(level=logging.INFO if verbose else logging.WARNING)

    for filename in filenames:
        before = time.time()
        ledger = load_file(filename)
        logger.info(
            "Loaded Beancount ledger (%s entries; %s errors) in %s",
            len(ledger.entries),
            len(ledger.errors),
            time.time() - before,
        )

        for error in ledger.errors:
            msg = click.style(error.message, fg="red")
            click.echo(f"{error.filename}:{error.line}:{msg}", err=True)


@cli.command()
@click.argument("filename", type=FILENAME_TYPE)
@click.option("-v", "--verbose", is_flag=True, help="Verbose output.")
@click.option(
    "--diff-balances",
    is_flag=True,
    help="Print out account balances where different.",
)
@click.option(
    "--print-options", is_flag=True, help="Print out the options from both."
)
def compare(
    *, filename: str, verbose: bool, print_options: bool, diff_balances: bool
) -> None:
    """Compare uro output for FILENAME to Beancunt.

    This loads the given file with both uromyces and Beancount and compares
    the output. Differences will be printed out.

    Some metadata fields (__tolerances__ and __automatic__) are ignored.
    """
    # Lazily import here to improve startup performance - in particular
    # the Fava one is slow.
    from beancount.core import data  # noqa: PLC0415
    from fava.core.tree import Tree  # noqa: PLC0415

    logging.basicConfig(level=logging.INFO if verbose else logging.WARNING)

    ledger = load_file(filename)
    entries_beancount, errors_beancount, options_beancount = load_beancount(
        filename
    )

    click.echo("Errors from uromyces")
    for error in ledger.errors:
        msg = click.style(error.message, fg="red")
        click.echo(f"{error.filename}:{error.line}:{msg}", err=True)
    click.echo("Errors from Beancount")
    for err in errors_beancount:
        msg = click.style(err.message, fg="red")
        source = getattr(err, "source", None)
        if source is not None:
            click.echo(
                f"{source['filename']}:{source['lineno']}:{msg}", err=True
            )
        else:
            click.echo(f"{msg}", err=True)

    entries_uromyces = data.sorted(beancount_entries(ledger.entries))

    if diff_balances:
        balances_beancount = {
            n.name: n.balance.to_strings()
            for n in Tree(entries_beancount).values()
        }
        balances_uromyces = {
            n.name: n.balance.to_strings()
            for n in Tree(entries_uromyces).values()
        }
        for account, balance_beancount in balances_beancount.items():
            balance_uromyces = balances_uromyces[account]
            if balance_beancount != balance_uromyces:
                click.echo(
                    click.style(
                        f"Found difference in account balance for {account}"
                        " between Beancount and uromyces:",
                        fg="red",
                    )
                )
                click.echo(balance_beancount)
                click.echo(balance_uromyces)

    if print_options:
        click.echo(click.style("Beancount options:", fg="green"))
        click.echo(pformat(options_beancount))
        click.echo(click.style("uromyces options:", fg="green"))
        click.echo(pformat(convert_options(ledger)))

    diff_count = 0
    for bc, uro in zip(entries_beancount, entries_uromyces, strict=True):
        if not compare_entries(bc, uro):
            diff_count += 1
            if diff_count >= 30:
                continue
            click.echo(
                click.style(
                    f"Found difference in entry on {uro.date}"
                    " between Beancount and uromyces:",
                    fg="red",
                )
            )
            click.echo(pformat(bc))
            click.echo(pformat(uro))

    if diff_count >= 30:
        click.echo(
            click.style(
                f"Found {diff_count} different entries,"
                " stopped printing after 30.",
                fg="red",
            )
        )
