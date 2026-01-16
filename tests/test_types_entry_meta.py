from __future__ import annotations

from collections.abc import ItemsView
from collections.abc import KeysView
from collections.abc import Mapping
from collections.abc import ValuesView

import pytest

from uromyces import EntryMeta
from uromyces import PostingMeta


def test_posting_meta() -> None:
    with pytest.raises(ValueError, match="Invalid filename"):
        PostingMeta({"filename": "not_a_path", "lineno": 0})
    with pytest.raises(TypeError, match="failed to extract enum MetaValue"):
        PostingMeta({"key": object()})  # type: ignore[dict-item]

    empty = PostingMeta({})
    assert empty.filename is None
    assert empty.lineno is None
    with pytest.raises(AttributeError):
        assert empty.not_an_attribute  # type: ignore[attr-defined]
    with pytest.raises(TypeError):
        assert PostingMeta(empty)

    with_filename = PostingMeta({"filename": "<dummy>", "lineno": 0})
    assert with_filename.filename == "<dummy>"
    assert with_filename.lineno == 0
    assert with_filename["lineno"] == 0
    assert isinstance(with_filename["lineno"], int)

    other_key = PostingMeta({"some_key": "test"})
    assert other_key["some_key"] == "test"
    assert other_key.get("some_key") == "test"
    assert other_key.get("other_key") is None
    assert other_key.get("other_key", "default") == "default"

    assert dict(empty) == {}
    assert dict(**empty) == {}
    assert dict(empty, key="value") == {"key": "value"}  # ty:ignore[no-matching-overload]

    assert dict(other_key) == {"some_key": "test"}
    assert dict(**other_key) == {"some_key": "test"}


def test_entry_meta() -> None:
    with pytest.raises(ValueError, match="Missing filename"):
        EntryMeta({})
    with pytest.raises(ValueError, match="Missing lineno"):
        EntryMeta({"filename": "<string>"})
    with pytest.raises(ValueError, match="Invalid filename"):
        EntryMeta({"filename": "not_a_path", "lineno": 0})

    EntryMeta({"filename": "<dummy>", "lineno": 0})
    EntryMeta({"filename": "/some/path", "lineno": 0})


def test_entry_meta_mapping() -> None:
    meta_dict: dict[str, str | int] = {
        "filename": "<string>",
        "lineno": 0,
        "key": "string",
    }
    header = EntryMeta(meta_dict)
    assert isinstance(header, Mapping)
    assert dict(header) == meta_dict
    assert dict(**header) == meta_dict

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


def test_entry_meta_constructor() -> None:
    header = EntryMeta({"filename": "<string>", "lineno": 0})
    # not an absolute path
    assert header.filename == "<string>"
    header = EntryMeta({"filename": "/home", "lineno": 0, "key": "string"})
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

    header = EntryMeta(
        {"filename": "/home", "lineno": 0, "__implicit_prices": "string"}
    )
