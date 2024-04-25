"""Python functionality to run Beancount plugins."""

from __future__ import annotations

import sys
import time
from importlib import import_module
from inspect import signature
from logging import info
from pathlib import Path
from traceback import format_exc
from typing import TYPE_CHECKING

from fava.helpers import BeancountError

from uromyces._convert import beancount_entries
from uromyces._convert import convert_options
from uromyces._convert import uromyces_entries

if TYPE_CHECKING:
    from typing import Any

    from uromyces import Ledger


CONVERT = False


def import_plugin(
    plugin: str,
) -> tuple[list[Any], list[BeancountError]]:
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
            BeancountError(
                None, f"`__plugins__` is missing in plugin '{plugin}'", None
            )
        ]
    except ImportError:
        return [], [
            BeancountError(None, f"Importing plugin '{plugin}' failed", None)
        ]


def run(ledger: Ledger) -> Ledger:  # noqa: C901
    """Run the Beancount plugins for the ledger."""
    plugin_errors = []
    if not ledger.plugins:
        info("No plugins to run.")
        return ledger
    entries = None
    options_map = convert_options(ledger)

    if ledger.options.insert_pythonpath:
        sys.path.insert(0, str(Path(ledger.filename).parent))
    for plugin in ledger.plugins:
        if ledger.run_plugin(plugin.name):
            # Rust implementation of the plugin
            continue
        if entries is None:
            before = time.time()
            entries = (
                beancount_entries(ledger.entries)
                if CONVERT
                else ledger.entries
            )
            if CONVERT:
                info(
                    "Converted all entries to Beancount in %s",
                    time.time() - before,
                )
        before = time.time()
        mod_plugins, errors = import_plugin(plugin.name)
        plugin_errors.extend(errors)
        for func in mod_plugins:
            sig = signature(func)
            conf_arg = () if len(sig.parameters) == 2 else (plugin.config,)  # noqa: PLR2004
            try:
                entries, new_errors = func(
                    entries,
                    options_map,
                    *conf_arg,
                )
                plugin_errors.extend(new_errors)
            except Exception:  # noqa: BLE001
                err = format_exc().replace("\n", "\n  ")
                plugin_errors.append(
                    BeancountError(
                        None,
                        f"Error running plugin '{plugin.name}': {err}",
                        None,
                    ),
                )
                continue
        info("Ran plugin %s in %s", plugin.name, time.time() - before)

    if entries is not None:
        before = time.time()
        entries = uromyces_entries(entries)
        info(
            "Convert any potential Beancount entries to uromyces in %s",
            time.time() - before,
        )
        ledger.replace_entries(entries)
    for error in plugin_errors:
        ledger.add_error(error)

    return ledger
