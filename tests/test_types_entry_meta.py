from __future__ import annotations

from collections.abc import ItemsView
from collections.abc import KeysView
from collections.abc import Mapping
from collections.abc import ValuesView
from datetime import date
from decimal import Decimal
from pathlib import Path

import pytest

from uromyces import Amount
from uromyces import EntryMeta
from uromyces import PostingMeta


def test_posting_meta() -> None:
    with pytest.raises(ValueError, match="Invalid filename"):
        PostingMeta({"filename": "not_a_path", "lineno": 0})
    with pytest.raises(TypeError, match="not a valid metadata value"):
        PostingMeta({"key": object()})  # type: ignore[dict-item]  # ty:ignore[invalid-argument-type]

    empty = PostingMeta({})
    assert empty == PostingMeta({})
    assert empty.filename is None
    assert empty.lineno is None
    with pytest.raises(AttributeError):
        assert empty.not_an_attribute  # type: ignore[attr-defined]  # ty:ignore[unresolved-attribute]
    with pytest.raises(TypeError):
        assert PostingMeta(empty)

    with_filename = PostingMeta({"filename": "<dummy>", "lineno": 0})
    assert with_filename == PostingMeta({"filename": "<dummy>", "lineno": 0})
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
    assert dict(empty, key="value") == {"key": "value"}

    assert dict(other_key) == {"some_key": "test"}
    assert dict(**other_key) == {"some_key": "test"}
    assert other_key == {"some_key": "test"}
    assert other_key.copy() == {"some_key": "test"}


def test_entry_meta() -> None:
    with pytest.raises(ValueError, match="Missing filename"):
        EntryMeta({})
    with pytest.raises(ValueError, match="Missing lineno"):
        EntryMeta({"filename": "<string>"})
    with pytest.raises(ValueError, match="Invalid filename"):
        EntryMeta({"filename": "not_a_path", "lineno": 0})

    EntryMeta({"filename": "<dummy>", "lineno": 0})
    EntryMeta({"filename": str(Path(__file__)), "lineno": 0})


def test_entry_meta_mapping() -> None:
    meta_dict: dict[str, str | int] = {
        "filename": "<string>",
        "lineno": 0,
        "key": "string",
    }
    header = EntryMeta(meta_dict)
    assert header == EntryMeta(meta_dict)
    assert isinstance(header, Mapping)
    assert dict(header) == meta_dict
    assert dict(**header) == meta_dict
    assert header.copy() == meta_dict

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
    home = str(Path.home())
    header = EntryMeta({"filename": home, "lineno": 0, "key": "string"})
    assert header.filename == home
    assert header["filename"] == home
    assert header["lineno"] == 0
    assert header["key"] == "string"
    assert list(header) == ["filename", "lineno", "key"]
    assert next(iter(header)) == "filename"
    assert "key" in header
    assert len(header) == 3
    with pytest.raises(KeyError, match="asdf"):
        header["asdf"]

    header = EntryMeta(
        {"filename": home, "lineno": 0, "__implicit_prices": "string"}
    )


def test_entry_meta_value_types() -> None:
    """Metadata values keep their Python type if there is a variant for it."""
    header = EntryMeta(
        {
            "filename": "<string>",
            "lineno": 0,
            "int": 5,
            "bool": True,
            "decimal": Decimal("5.50"),
            "date": date(2020, 1, 1),
            "string": "x",
            "amount": Amount(Decimal("1.00"), "USD"),
        }
    )
    for key, expected in (
        ("int", 5),
        ("bool", True),
        ("decimal", Decimal("5.50")),
        ("date", date(2020, 1, 1)),
        ("string", "x"),
        ("amount", Amount(Decimal("1.00"), "USD")),
    ):
        assert header[key] == expected
        assert type(header[key]) is type(expected)

    # integers that do not fit into an i64 fall back to being decimals
    too_large = EntryMeta({"filename": "<string>", "lineno": 0, "int": 2**80})
    assert too_large["int"] == Decimal(2**80)

    # a key without a value (`key:` in a Beancount file) is `None`
    no_value = EntryMeta(
        {"filename": "<string>", "lineno": 0, "key": None}  # type: ignore[dict-item]  # ty:ignore[invalid-argument-type]
    )
    assert no_value["key"] is None
    assert no_value.get("key") is None
    assert "key" in no_value
    assert dict(no_value) == {
        "filename": "<string>",
        "lineno": 0,
        "key": None,
    }
