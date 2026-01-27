# Beancount Syntax

This document covers the Beancount syntax supported by uromyces.

See also:

- [The official Beancount syntax documentation](http://furius.ca/beancount/doc/syntax)
- [The tree-sitter grammar used by uromyces](https://github.com/yagebu/tree-sitter-beancount)

## Basic Elements

### Comments

Inline comments start with `;`, and lines starting with any of the characters
`*:!&#?%;` are also considered as comments and skipped by the parser.

```beancount
2012-12-12 open Assets:Cash ; This is an inline comment
* This is also a comment
```

### Strings

Free-form text (transaction payees, narrations, notes, ...) are included as
strings between double quotes (`"`). These can span multiple lines and contain
arbitrary unicode characters. If your string is supposed to contain a double
quote, that can be achieved by escaping it with a backslash (`\"`).

```beancount
2024-01-15 note Assets:Cash "A string"
2024-01-15 note Assets:Cash "A string with a quote \" inside"
2024-01-15 note Assets:Cash "A string
over two lines"
```

### Dates

All dated directives are prefixed with a date in `YYYY-MM-DD` format:

```beancount
2024-01-15 open Assets:Cash
```

### Accounts

The commodities you track with Beancount are accumulated in a hierarchy of
accounts. Accounts are given by a colon-separated list of capitalized words.
They must begin with one of the five root accounts listed in the table below.
The separation by colons defines an implicit hierarchy, for example we say that
`Assets:Cash` is a sub-account of `Assets`.

| Name          | Type | Contains                     | Examples                  |
| ------------- | ---- | ---------------------------- | ------------------------- |
| `Assets`      | +    | Cash, Checking-Account, etc. | `Assets:Checking`         |
| `Liabilities` | -    | Credit Card, etc.            | `Liabilities:CreditCard`  |
| `Income`      | -    | Salary, etc.                 | `Income:EmployerA`        |
| `Expenses`    | +    | Expense categories           | `Expenses:Fun:Cinema`     |
| `Equity`      | -    | Almost always auto-generated | `Equity:Opening-Balances` |

The root account names should start with a capital letter, followed by any
letters, numbers, or dashes (`-`). Subsequent account components should start
with a capital letter or number, again followed by any letters, numbers, or
dashes.

The names of the five root accounts can be changed with the following options:

```beancount
option "name_assets"      "Vermoegen"
option "name_liabilities" "Verbindlichkeiten"
option "name_income"      "Einkommen"
option "name_expenses"    "Ausgaben"
option "name_equity"      "Eigenkapital"
```

### Currencies

Currencies (also called commodities) are strings which

- Begins with either a letter or a slash (/) character.
- Contains at least one letter.
- Ends with either a letter or a number.
- May contain the following special characters within: period (`.`), underscore
  (`\_`), dash (`-`), or quote (`'`).

Some examples:

- `AAPL` - stock
- `V` - single-character stock
- `NT.TO` - stock on another market
- `TLT_040921C144` - equity option
- `/6J` - currency futures
- `/NQH21` - commodity futures
- `/NQH21_QNEG21C13100` - futures option

### Numbers

Decimal numbers are written with a period as decimal separator and can include
commas as thousands separators. You can also write numerical expressions using
addition, substraction, division and multiplication:

```beancount
1000.00
1,000.00
-500.50
(28 + 8) / 8
```

## Directives

Most data you provide in your Beancount ledger comes in the form of so-called
directives that are dated and usually refer to some accounts. You do not need
to provide them in a sorted order, they will be sorted by date by uromyces
before handling.

### Open

Opens an account. Accounts need to be opened (datewise) before being used.
Optionally specify allowed currencies and a booking method.

```beancount
2020-01-01 open Assets:Bank:Checking

; With currency constraint - using any currency different from `USD` on this account will error.
2020-01-01 open Assets:Cash USD

; With multiple currencies - only the comma-separated list of currencies is allowed.
2020-01-01 open Assets:Brokerage USD,EUR,AAPL

; With booking method
2020-01-01 open Assets:Brokerage USD "FIFO"
```

Available booking methods:

- `STRICT` (default)
- `NONE`
- `AVERAGE`
- `FIFO`
- `LIFO`
- `HIFO`
- `STRICT_WITH_SIZE`

### Close

Closes an account. Referencing this account on a date after it was closed
causes errors (except for Balance, Document, and Note directives, which are
also allowed for already closed accounts).

```beancount
2024-12-31 close Assets:Bank:OldAccount
```

### Commodity

Declares a commodity/currency and attaches metadata to it. It is not necessary
to declare all commodities but recommended.

```beancount
2024-01-01 commodity USD
  name: "US Dollar"

2024-01-01 commodity AAPL
  name: "Apple Inc."
  asset-class: "stock"
```

### Balance

Asserts the balance of an account at the beginning of a day.

```beancount
2024-01-01 balance Assets:Bank:Checking 1500.00 USD
```

Balance assertions with tolerance:

```beancount
2024-01-01 balance Assets:Bank:Checking 1500.00 ~ 0.01 USD
```

### Pad

Automatically inserts a transaction to pad an account to match a subsequent
balance assertion.

```beancount
2024-01-01 pad Assets:Bank:Checking Equity:Opening-Balances
2024-01-02 balance Assets:Bank:Checking 1000.00 USD
```

### Transactions

Transactions record the movement of money between accounts. The sum of all
postings must equal zero.

```beancount
; Basic transaction with narration only
2024-01-15 * "Grocery shopping"
  Expenses:Food:Groceries    50.00 USD
  Assets:Cash               -50.00 USD

; With payee and narration
2024-01-15 * "Whole Foods" "Weekly groceries"
  Expenses:Food:Groceries    75.00 USD
  Assets:Bank:Checking

; Using 'txn' keyword
2024-01-15 txn "Transfer"
  Assets:Bank:Savings       500.00 USD
  Assets:Bank:Checking     -500.00 USD
```

#### Transaction Flags

- `*` - Completed/cleared transaction
- `!` - Pending/flagged transaction
- `txn` - Equivalent to `*`
- Other ASCII characters from `A` to `Z` are also allowed as flags.

```beancount
2024-01-15 * "Cleared transaction"
  Expenses:Food    20.00 USD
  Assets:Cash

2024-01-15 ! "Pending transaction"
  Expenses:Food    20.00 USD
  Assets:Cash
```

#### Posting Flags

Individual postings can also be flagged:

```beancount
2024-01-15 * "Transaction with flagged posting"
  ! Expenses:Food    20.00 USD
  Assets:Cash
```

### Note

Attaches a note to an account on a specific date.

```beancount
2024-01-15 note Assets:Bank:Checking "Called bank about fees"
```

### Document

Links a document file to an account.

```beancount
2024-01-15 document Assets:Bank:Checking "/path/to/statement.pdf"
```

### Price

Records the price of a commodity in another currency.

```beancount
2024-01-15 price AAPL 185.50 USD
2024-01-15 price EUR 1.08 USD
```

### Event

Records a dated event (useful for tracking life events, locations, etc.).

```beancount
2024-01-15 event "location" "New York, USA"
2024-01-15 event "employer" "Acme Corp"
```

### Query

Defines a named BQL query.

```beancount
2024-01-01 query "expenses-by-account" "
  SELECT account, sum(position)
  WHERE account ~ 'Expenses'
  GROUP BY account
"
```

### Custom

Defines custom directives that can be used by other tools (e.g. by Fava).

```beancount
2024-01-15 custom "budget" Expenses:Food 500 USD
2024-01-15 custom "fava-option" "language" "en"
```

## Undated Directives

There are also undated directives that allow you to configure uromyces
behaviour, specify plugins to be used and include other Beancount files.

### Option

Sets options for the ledger.

```beancount
option "title" "My Personal Finances"
option "operating_currency" "USD"
option "booking_method" "FIFO"
```

### Plugin

Loads a plugin module.

```beancount
plugin "beancount.plugins.auto_accounts"
plugin "beancount.plugins.forecast" "30"
```

### Include

Includes another Beancount file.

```beancount
include "accounts.beancount"
include "2024/*.beancount"
```

### Pushtag and poptag

All directives between these two directives will have the given tag. So e.g.
the following

```beancount
pushtag #my-tag

2024-01-15 * "Cleared transaction"
  Expenses:Food    20.00 USD
  Assets:Cash

poptag #my-tag
```

results in the same output as

```beancount
2024-01-15 * "Cleared transaction" #my-tag
  Expenses:Food    20.00 USD
  Assets:Cash
```

### Pushmeta and popmeta

This is like pushtag / poptag but for metadata:

```beancount
pushmeta key: "value"

2024-01-15 * "Cleared transaction"
  Expenses:Food    20.00 USD
  Assets:Cash

popmeta key:
```

## Costs and Prices

### Cost Specification

Use `{}` to specify the cost basis of a position:

```beancount
; Buying shares at a specific cost
2024-01-15 * "Buy stock"
  Assets:Brokerage    10 AAPL {185.00 USD}
  Assets:Bank:Checking

; With acquisition date
2024-01-15 * "Buy stock"
  Assets:Brokerage    10 AAPL {185.00 USD, 2024-01-15}
  Assets:Bank:Checking

; With label
2024-01-15 * "Buy stock"
  Assets:Brokerage    10 AAPL {185.00 USD, "lot1"}
  Assets:Bank:Checking

; Total cost
2024-01-15 * "Buy stock"
  Assets:Brokerage    10 AAPL {# 1850.00 USD}
  Assets:Bank:Checking
```

### Price Annotation

Use `@` for per-unit price or `@@` for total price:

```beancount
; Per-unit price
2024-01-15 * "Currency exchange"
  Assets:EUR    100 EUR @ 1.08 USD
  Assets:USD   -108 USD

; Total price
2024-01-15 * "Currency exchange"
  Assets:EUR    100 EUR @@ 108 USD
  Assets:USD   -108 USD
```

### Reducing Positions

Use `{}` to match existing lots when selling:

```beancount
; Sell at specific cost
2024-06-15 * "Sell stock"
  Assets:Brokerage   -5 AAPL {185.00 USD}
  Assets:Bank:Checking  1000 USD
  Income:CapitalGains

; Let booking method choose (empty cost spec)
2024-06-15 * "Sell stock"
  Assets:Brokerage   -5 AAPL {}
  Assets:Bank:Checking  1000 USD
  Income:CapitalGains
```

## Tags and Links

You can tag all kinds of directives in uromyces (in Beancount, only documents,
notes, and transactions can have tags and links). These can be used to filter
matching directives in plugins or when generating reports. Tags start with `#`,
links start with `^`:

```beancount
2024-01-15 * "Business lunch" #work #tax-deductible ^invoice-2024-001
  Expenses:Food:Business    45.00 USD
  Assets:Bank:Checking
```

Tags and links can consist of any letter (uppercase or lowercase) from `A` to
`Z`, digits (`0`-`9`), dashes (`-`), underscores (`_`), periods (`.`), and
forward slashes (`/`).

## Metadata

Metadata can be attached to directives and postings as key-value pairs on
indented lines:

```beancount
2024-01-15 * "Grocery shopping"
  memo: "Weekly groceries"
  receipt: "/receipts/2024-01-15.jpg"
  Expenses:Food:Groceries    50.00 USD
    category: "produce"
  Assets:Cash
```

Metadata values can be strings, numbers, dates, currencies, booleans, amounts,
or accounts:

```beancount
2024-01-01 commodity USD
  name: "US Dollar"
  export: TRUE
  inception: 1792-01-01
  precision: 2
```

## Automatic Balance Completion

One posting per transaction can omit its amount, which will be automatically
computed:

```beancount
2024-01-15 * "Grocery shopping"
  Expenses:Food:Groceries    50.00 USD
  Assets:Cash  ; Amount automatically computed as -50.00 USD
```
