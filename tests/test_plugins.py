from __future__ import annotations

from beancount.plugins.auto_accounts import auto_insert_open

from uromyces._plugins import import_plugin


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
