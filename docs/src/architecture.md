# Architecture

This page gives an overview over the internal architecture of uromyces.
uromyces is a Rust implementation of Beancount's plain text accounting
functionality, exposed as a Python library via [PyO3](https://pyo3.rs/).

## Goals

- Improved performance compared to Beancount.
- Better separation of unbooked and booked directives.
- Implementation of features that never made it into Beancount v3.
- Transparent documentation of features and implementation details.
- Compatibility with Beancount's plugin ecosystem.

## Processing Pipeline

uromyces follows a 5-stage pipeline that mirrors Beancount's architecture:

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌────────────┐
│  Parse  │ -> │ Combine │ -> │ Booking │ -> │ Plugins │ -> │ Validation │
└─────────┘    └─────────┘    └─────────┘    └─────────┘    └────────────┘
```

### Stage 1: Parsing individual Beancount files

**Location:** `src/parse/`

The parse stage converts Beancount text files into Rust data structures using a
tree-sitter grammar. This operates per file, so multiple files could be parsed
in parallel and on changes, only changed files need to be reparsed.

- `mod.rs`: Parser initialization and orchestration using the tree-sitter C
  library.
- `convert.rs`: Converts tree-sitter tree nodes to Rust types.
- `node_fields.rs` / `node_ids.rs`: Parser constants synchronized with
  `parser.c` during build .
- `errors.rs`: Parse-specific errors.

**Output:** `ParsedFile` containing raw entries and any errors.

### Stage 2: Combine

**Location:** `src/combine.rs`

The combine stage merges multiple parsed files by processing `include`
directives.

- `load()`: Main entry point that loads a file plus all includes and performs
  booking.
- `load_string()`: Alternative entry for string input.
- `load_beancount_file()`: Recursively processes include directives while
  preventing circular includes.

**Output:** `RawLedger` containing unsorted raw entries from all included
files.

### Stage 3: Booking

**Location:** `src/booking/`

The booking stage matches postings with existing lots in accumulated
inventories using configurable booking methods.

- `mod.rs`: Main booking orchestration via `book_entries()`.
- `methods.rs`: Implementation of different booking strategies.
- `currency_groups.rs`: Currency resolution and gap-filling logic.
- `errors.rs`: Booking-specific error types.

Key data structures:

- `Inventory`: Maps `(Currency, Option<Cost>)` to decimal quantities for
  tracking account positions.

**Output:** `Ledger` with fully booked transactions and balance assertions.

### Stage 4: Plugins

**Location:** `src/plugins/` and `python/uromyces/_plugins.py`

Uromyces uses a two-layer plugin system:

**Rust Pre-Plugins** (run automatically after booking):

- `documents.rs`: Finds document entries
- `pad.rs`: Creates padding transactions for balance assertions

**Named Plugins** (Rust-accelerated with Python fallback):

- `implicit_prices.rs`: Extracts prices from transaction costs
- The system checks for a Rust implementation first; if unavailable, it falls
  back to executing the Python plugin

The Python plugin orchestrator (`_plugins.py`) handles:

- Attempting pure Rust implementations first
- Converting entries to Beancount namedtuples when Python plugins are needed
- Executing standard Beancount plugins via dynamic import
- Converting results back to uromyces types

### Stage 5: Validation

**Location:** `src/plugins/validation.rs`

Built-in validators check ledger consistency:

- `account_names`: Validates account hierarchy.
- `open_close`: Ensures accounts are opened before use and closed correctly.
- `duplicate_balances`: Detects multiple balance assertions on the same
  date/account.
- `duplicate_commodities`: Checks commodity uniqueness.
- `active_accounts`: Verifies only opened accounts are used.
- `currency_constraints`: Validates currency compatibility.
- `transaction_balances`: Checks that transactions balance in each currency.
- `check_balance_assertions`: Validates account balances match assertions.

All validators accumulate errors without stopping on the first error.

## Type System

### Raw vs Booked Types

Types exist in two forms representing pre/post-booking states:

| Stage          | Entry Type | Transaction Type | Fields                    |
| -------------- | ---------- | ---------------- | ------------------------- |
| Parse/Combine  | `RawEntry` | `RawTransaction` | Optional/partial amounts  |
| Booking onward | `Entry`    | `Transaction`    | Complete/resolved amounts |

**RawPosting** contains optional amounts and potentially incomplete cost
specifications:

```rust
pub struct RawPosting {
    pub account: Account,
    pub units: RawAmount,        // Option<Decimal>, Option<Currency>
    pub cost: Option<CostSpec>,  // Potentially incomplete
    pub price: Option<RawAmount>,
    // ...
}
```

**Posting** (after booking) has all fields fully resolved:

```rust
pub struct Posting {
    pub account: Account,
    pub units: Amount,           // Always has number & currency
    pub cost: Option<Cost>,      // Fully specified
    pub price: Option<Amount>,
    // ...
}
```

### String Interning

For memory efficiency, frequently repeated strings are interned using the
`internment` crate with `ArcIntern<String>`:

- `Currency`: e.g., "USD" stored once in memory
- `Account`: Parsed into a hierarchy with `.parent()` traversal support

String comparison becomes pointer comparison (O(1) vs O(n)).

## Source Directory Structure

This follows the layout suggested by
[maturin](https://www.maturin.rs/project_layout.html#mixed-rustpython-project),
with a directory for the Rust source code under `src/` and the Python source
code in `python/uromyces/`:

```
src/
├── parse/                  # Stage 1: Tree-sitter grammar parsing
├── combine.rs              # Stage 2: File loading & include handling
├── booking/                # Stage 3: Lot matching algorithms
├── plugins/                # Stage 4 & 5: Plugins and validators
├── types/                  # Core data types (Account, Amount, Cost, Entry, etc.)
├── ledgers.rs              # RawLedger & Ledger structs
├── inventory.rs            # Position tracking for booking
├── options.rs              # BeancountOptions parsing
├── errors.rs               # UroError type with metadata
└── lib.rs                  # PyO3 module definition

python/uromyces/
├── __init__.py             # Public API
├── _uromyces.pyi           # Type stubs for the compiled module
├── _plugins.py             # Python plugin orchestration
├── _convert.py             # Type conversion (uromyces ↔ Beancount)
└── _cli.py                 # CLI commands (check, compare)
```

## Build System

Uromyces uses [maturin](https://www.maturin.rs/) to compile Rust code as a
Python extension module.

1. **Build Script** (`build.rs`):

   - Extracts and updates constants for node IDs and field names in
     `src/parse/` from the tree-sitter parser.
   - Compiles the tree-sitter grammar (`parser.c` and `scanner.c`) into a
     static library.

1. **Maturin** (configured in `pyproject.toml`):

   - Builds the Rust code as a Python extension module
   - Produces `_uromyces.abi3.so` (ABI3 for Python 3.10+ compatibility)

1. **PyO3 Module** (`src/lib.rs`):

   - Exports Rust types and functions to Python
   - Registers type mappings for Python interop

## Rust-Python Interaction

### Type Conversion

Each entry type has a `._convert()` method (defined in Rust via PyO3) to
convert to Beancount namedtuples. The `_convert.py` module handles
bidirectional conversion:

```python
# Uromyces → Beancount
def beancount_entries(entries):
    return [entry._convert() for entry in entries]

# Beancount → Uromyces (via singledispatch)
@beancount_to_uromyces.register(data.Balance)
def _(entry: data.Balance) -> Balance:
    return Balance(entry.meta, entry.date, entry.account, ...)
```

### Plugin Execution

```python
def load_file(filename):
    ledger = _uromyces.load_file(filename)  # Rust: Parse + Combine + Booking
    run(ledger)                              # Python: Plugin orchestration
    ledger.run_validations()                 # Rust: Built-in validators
    return ledger
```

For each plugin:

1. Try Rust implementation: `ledger.run_plugin(plugin.name)` returns `True` if
   handled
1. If `False`: Convert entries to Beancount format, execute Python plugin,
   convert back
