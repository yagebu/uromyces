"""Python functionality to run Beancount plugins."""

from __future__ import annotations

from importlib import import_module
from logging import getLogger
from pathlib import Path
from traceback import format_exc
from typing import NamedTuple
from typing import TYPE_CHECKING

from uromyces._convert import beancount_entries
from uromyces._convert import convert_options
from uromyces._convert import uromyces_entries
from uromyces._util import insert_sys_path
from uromyces._util import log_timing

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

    from beancount.core import data

    from uromyces import Directive
    from uromyces import Ledger


logger = getLogger(__name__)


class PluginError(NamedTuple):
    """Error when trying to run a plugin."""

    source: None
    message: str
    entry: None = None

    @staticmethod
    def from_exception(message: str) -> PluginError:
        err = format_exc().replace("\n", "\n  ")
        return PluginError(
            None,
            f"{message}:\n\n{err}",
        )


def import_plugin(
    plugin: str,
) -> tuple[list[Any], list[PluginError]]:
    """Try importing a plugin and load the list of functions."""
    try:
        module = import_module(plugin)
        mod_plugins = getattr(module, "__plugins__", None)
        if isinstance(mod_plugins, (list, tuple)):
            functions = [
                getattr(module, func) if isinstance(func, str) else func
                for func in mod_plugins
            ]
            return functions, []
        return [], [
            PluginError(None, f"`__plugins__` is missing in plugin '{plugin}'")
        ]
    except ImportError:
        return [], [
            PluginError.from_exception(f"Importing plugin '{plugin}' failed")
        ]


def run(ledger: Ledger) -> None:
    """Run the Beancount plugins for the ledger.

    Will try to run pure Rust implementations of plugins via ledger.run_plugin.
    If this is not possible, the entries will be converted to Beancount entries
    according to the convert paramater and all subsequent plugins will run via
    their Python implementation.

    Args:
        ledger: The ledger to run the plugins on.
        convert: Whether to convert the entries to Beancount namedtuples.
    """
    plugins = ledger.plugins
    plugin_errors = []
    if not plugins:
        logger.info("No plugins to run.")
        return
    entries: Sequence[Directive | data.Directive] | None = None
    options_map = convert_options(ledger)

    with insert_sys_path(
        Path(ledger.filename).parent
        if ledger.options.insert_pythonpath
        else None
    ):
        for plugin in plugins:
            if entries is None:
                if ledger.run_plugin(plugin.name):
                    # Rust implementation of the plugin
                    continue
                entries = ledger.entries
                with log_timing(
                    logger, "convert all uromyces entries to Beancount"
                ):
                    entries = beancount_entries(entries)
            with log_timing(logger, f"plugin '{plugin.name}' (Python)"):
                mod_plugins, errors = import_plugin(plugin.name)
                plugin_errors.extend(errors)
                for func in mod_plugins:
                    conf_arg = (
                        () if plugin.config is None else (plugin.config,)
                    )
                    try:
                        entries, new_errors = func(
                            entries,
                            options_map,
                            *conf_arg,
                        )
                        plugin_errors.extend(new_errors)
                    except Exception:  # noqa: BLE001
                        plugin_errors.append(
                            PluginError.from_exception(
                                f"Error running plugin '{plugin.name}'"
                            )
                        )
                        continue

    if entries is not None:
        with log_timing(logger, "convert any Beancount entries to uromyces"):
            entries = uromyces_entries(entries)
        ledger.replace_entries(entries)
    for error in plugin_errors:
        ledger.add_error(error)
