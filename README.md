# urom(yces)

urom is just a toy project to try out the Rust language to implement parts
of Beancount's functionality.

## Components

Just like Beancount, this tries to go from an input file to an usable result of entries.

It does so in the following series of steps.

1. [x] Parse single files with a tree-sitter grammar to obtain abstract syntax trees.
2. [x] Convert the syntax tree to produce Rust data structures.
3. [x] Combine the parsed results from multiple files and run initial validations.
4. [x] Booking
5. [ ] Plugins, ...
6. [x] Validation

## Differences to Beancount V2

- The automatic filling of missing currencies is stricter (less powerful) than the one by
  Beancount and does not take the account balances into account. IMHO leaving out currencies
  should be discouraged and making this depend on the previous account balance seems error-prone.
- Likewise, the interpolation is less powerful. For example it won't be able to interpolate a
  missing total cost. Interpolating total cost seems to be rather an edge case anyway.
- The (deprecated) total cost syntax ({{}}) is not supported.
- Deprecated options are not supported.

## Etymology

The name is derived from the genus of rust fungi that can befall bean plants.
