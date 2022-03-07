from typing import Any

class Transaction:
    pass

class Ledger:
    entries: list[Any]

def load_file(filename: str) -> Ledger: ...
def convert_entries(ledger: Ledger) -> list[Any]: ...
