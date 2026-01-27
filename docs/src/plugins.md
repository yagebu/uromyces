# Plugins

## Pre-plugins

Pre-plugins run automatically after booking, before any user-specified plugins.
They are independent of each other and handle core Beancount functionality.

### `documents`

The documents plugin automatically discovers document files from configured
document directories. For each directory specified in the `option "documents"`
directive, it:

1. Scans subdirectories matching account names (e.g., `Assets/Bank/Checking/`)
1. Finds files whose names start with a valid date (e.g.,
   `2024-01-15-statement.pdf`)
1. Creates `Document` entries for each matching file

This allows you to organize receipts and statements in folders matching your
account hierarchy, and have them automatically linked to your ledger.

### `pad`

The pad plugin processes `pad` directives by generating synthetic transactions
to reconcile account balances. When you write:

```beancount
2024-01-01 pad Assets:Bank:Checking Equity:Opening-Balances
2024-01-15 balance Assets:Bank:Checking 1000.00 USD
```

The plugin:

1. Tracks the running balance of padded accounts
1. At each `balance` assertion, calculates the difference between the current
   balance and the asserted amount
1. Inserts a padding transaction (with flag `P`) dated on the pad directive's
   date to make up the difference

This is useful for setting opening balances or correcting discrepancies without
manually calculating the exact amounts

## "Normal" plugins

## Validation plugins

Validation plugins run after all other plugins and perform read-only checks on
the ledger, emitting errors for any issues found.

### `account_names`

Validates that all account names:

- Start with one of the configured root accounts (Assets, Liabilities, Equity,
  Income, Expenses by default)
- Match the valid naming pattern (components start with uppercase letter or
  digit, contain only letters, digits, and hyphens)

### `open_close`

Checks the consistency of account lifecycle:

- Each account is opened at most once
- Each account is closed at most once
- Only previously opened accounts can be closed

### `duplicate_balances`

Detects conflicting balance assertions: if two `balance` directives exist for
the same account, date, and currency but with different amounts, an error is
raised.

### `duplicate_commodities`

Ensures each currency has at most one `commodity` directive.

### `active_accounts`

Verifies that accounts are only used while active (between their `open` and
`close` dates). Exceptions are made for `note`, `balance`, and `document`
entries, which may reference closed accounts.

### `currency_constraints`

For accounts that declare allowed currencies in their `open` directive (e.g.,
`open Assets:Bank USD, EUR`), validates that only those currencies appear in
transactions and balance assertions for that account.

### `transaction_balances`

Checks that every transaction balances to zero (debits equal credits), within
the configured tolerance for each currency.

### `check_balance_assertions`

Verifies that `balance` assertions match the actual accumulated balance of the
account at that point in time. Reports the difference when assertions fail.

### `document_files_exist`

Confirms that all files referenced by `document` directives actually exist on
the filesystem
