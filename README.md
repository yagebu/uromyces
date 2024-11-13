# urom(yces)

urom is just a toy project to try out the Rust language to implement parts of
Beancount's functionality.

## How to use / run

You can use the provided Makefile to set up a virtualenv (at `.venv`) and
install uromyces in it with `make dev` and then try out e.g.
`uro -v $BEANCOUNT_FILE` to run a bean-check like script that will do a full
parse and print out any errors.

For more elaborate playing around it's probably best to write a Python script
that uses the `uromyces.load_file` function.

## Components

Just like Beancount, this tries to go from an input file to an usable result of
entries.

It does so in the following series of steps.

1. Parse single files with a tree-sitter grammar to obtain abstract syntax
   trees.
1. Convert the syntax tree to produce Rust data structures.
1. Combine the parsed results from multiple files and run initial validations.
1. Booking
1. Plugins
1. Validation

## Performance

### Parsing

There's quite some room for improvement in the parser surely, it hasn't been
the focus so far. Since the parser does more work per-file, parallelisation
with rayon maybe could give nice speed-ups for users who have multiple
includes. There's a couple other Rust parser implementations for Beancount
already, maybe one of them could be adapted as well.

### Imcremental computation

In the context of Fava, using salsa-rs ("A generic framework for on-demand,
incrementalized computation.") seems like a good fit to speed up the
incremental re-parses and more of a modified ledger.

## Differences to Beancount (V2)

- Not a lot of attention has been placed on generating the same kinds of
  errors. So, e.g., for a transaction that does not balance, the error messages
  from Beancount might be quite different.
- The automatic filling of missing currencies is stricter (less powerful) than
  the one by Beancount and only takes the account balances into account to
  infer cost currencies. IMHO leaving out currencies should be discouraged and
  making this depend on the previous account balance seems error-prone.
- Likewise, the interpolation is less powerful. For example it won't be able to
  interpolate a missing total cost. Interpolating total cost seems to be rather
  an edge case anyway.
- The (deprecated) total cost syntax (`{{}}`) is not supported.
- Deprecated options are not supported.
- The options `account_rounding`, `infer_tolerance_from_cost`, and
  `plugin_processing_mode` are not supported.

## Etymology

The name is derived from the genus of rust fungi that can befall bean plants.
