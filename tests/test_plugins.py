from __future__ import annotations

from typing import TYPE_CHECKING

from beancount.plugins.auto_accounts import auto_insert_open

from uromyces._plugins import import_plugin

if TYPE_CHECKING:
    from uromyces import Ledger


_name_of_a_function = "not a function"

__plugins__ = ["_name_of_a_function"]


def test_import_plugin_non_existent() -> None:
    plugins, errors = import_plugin("non_existent")
    assert not plugins
    assert errors
    assert errors[0].message.startswith(
        "Importing plugin 'non_existent' failed:\n\nTraceback"
    )


def test_import_plugin_no__plugins__() -> None:
    plugins, errors = import_plugin("uromyces")
    assert not plugins
    assert errors
    assert errors[0].message.startswith(
        "`__plugins__` is missing in plugin 'uromyces'"
    )


def test_import_plugin_beancount() -> None:
    plugins, errors = import_plugin("beancount.plugins.auto_accounts")
    assert plugins == [auto_insert_open]
    assert not errors


def test_plugin_in_testdir(load_doc: Ledger) -> None:
    """
    plugin "test_types"
    """
    errors = load_doc.errors
    assert errors
    assert errors[0].message.startswith("Importing plugin 'test_types' failed")


def test_plugin_rust(load_doc: Ledger) -> None:
    """
    plugin "beancount.plugins.implicit_prices"
    """
    assert not load_doc.errors


def test_insert_pythonpath(load_doc: Ledger) -> None:
    """
    option "insert_pythonpath" "True"
    ; This imports this file itself...
    plugin "beancount.plugins.implicit_prices"
    plugin "test_plugins"
    plugin "test_plugins"
    """
    errors = load_doc.errors
    assert errors
    assert "'str' object is not callable" in errors[0].message


def test_insert_pythonpath_no_plugins(load_doc: Ledger) -> None:
    """
    option "insert_pythonpath" "True"
    plugin "test_types"
    plugin "beancount.plugins.implicit_prices"
    """
    errors = load_doc.errors
    assert errors
    assert errors[0].message.startswith(
        "`__plugins__` is missing in plugin 'test_types'"
    )


def test_insert_pythonpath_not_found(load_doc: Ledger) -> None:
    """
    option "insert_pythonpath" "True"
    plugin "t"
    """
    errors = load_doc.errors
    assert errors
    assert errors[0].message.startswith("Importing plugin 't' failed")
