from __future__ import annotations

from collections.abc import ItemsView
from collections.abc import KeysView
from collections.abc import Mapping
from collections.abc import ValuesView
from datetime import date

import pytest

from uromyces import EntryHeader


def test_entry_header() -> None:
    date_ = date(2022, 12, 12)
    with pytest.raises(ValueError, match="Missing filename"):
        EntryHeader({}, date_, {"asdf"})
    with pytest.raises(ValueError, match="Missing lineno"):
        EntryHeader({"filename": "<string>"}, date_, {"asdf"})
    with pytest.raises(ValueError, match="Invalid filename"):
        EntryHeader({"filename": "not_a_path", "lineno": 0}, date_, {"asdf"})

    EntryHeader({"filename": "<dummy>", "lineno": 0}, date_, {"asdf"})
    EntryHeader({"filename": "/some/path", "lineno": 0}, date_, {"asdf"})


def test_entry_header_mapping() -> None:
    meta_dict: dict[str, str | int] = {
        "filename": "<string>",
        "lineno": 0,
        "key": "string",
    }
    header = EntryHeader(
        meta_dict,
        date(2022, 12, 12),
        {"asdf"},
    )
    assert isinstance(header, Mapping)
    assert dict(header) == meta_dict

    assert header["filename"] == "<string>"
    assert header.get("filename") == "<string>"

    with pytest.raises(KeyError):
        assert header["not_a_key"]
    assert header.get("not_a_key") is None
    assert header.get("not_a_key", "asdf") == "asdf"

    keys = header.keys()
    assert isinstance(keys, KeysView)
    assert "filename" in keys
    assert list(header.keys()) == list(meta_dict.keys())

    values = header.values()
    assert isinstance(values, ValuesView)
    assert list(header.values()) == list(meta_dict.values())

    items = header.items()
    assert isinstance(items, ItemsView)
    assert list(header.items()) == list(meta_dict.items())


def test_entry_header_constructor() -> None:
    header = EntryHeader(
        {"filename": "<string>", "lineno": 0},
        date(2022, 12, 12),
        {"asdf"},
    )
    assert header.tags == frozenset({"asdf"})
    assert header.links == frozenset()
    # not an absolute path
    assert header.filename == "<string>"
    header = EntryHeader(
        {"filename": "/home", "lineno": 0, "key": "string"},
        date(2022, 12, 12),
        {"asdf"},
    )
    assert header.filename == "/home"
    assert header["filename"] == "/home"
    assert header["lineno"] == 0
    assert header["key"] == "string"
    assert list(header) == ["filename", "lineno", "key"]
    assert next(iter(header)) == "filename"
    assert "key" in header
    assert len(header) == 3
    with pytest.raises(KeyError, match="asdf"):
        header["asdf"]

    header = EntryHeader(
        {"filename": "/home", "lineno": 0, "__implicit_prices": "string"},
        date(2022, 12, 12),
        {"asdf"},
    )
