"""Test CLI."""

from __future__ import annotations

from typing import TYPE_CHECKING

from click.testing import CliRunner

from uromyces._cli import cli

if TYPE_CHECKING:
    from pathlib import Path


def test_cli(test_ledgers_dir: Path) -> None:
    """Some basic checks that the CLI works."""
    runner = CliRunner()
    filename = str(test_ledgers_dir / "example.beancount")

    without_args = runner.invoke(cli)
    assert without_args.exit_code > 0

    without_filename = runner.invoke(cli, ("check"))
    assert without_filename.exit_code > 0
    assert "No file specified" in without_filename.output

    with_filename = runner.invoke(cli, ("check", filename))
    assert with_filename.exit_code == 0
    assert not with_filename.output

    with_filename_errors = runner.invoke(
        cli, ("check", str(test_ledgers_dir / "invalid-input.beancount"))
    )
    assert with_filename_errors.exit_code == 1

    compare = runner.invoke(cli, ("compare", filename))
    assert compare.exit_code == 0
