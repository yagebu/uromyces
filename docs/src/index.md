# Introduction

uromyces is a Rust re-implementation of Beancount's functionality.

## How to use / run

You can use the provided Makefile to set up a virtualenv (at `.venv`) and
install uromyces in it with `make dev` and then try out e.g.
`uro check -v $BEANCOUNT_FILE` to run a bean-check like script that will do a
full parse and print out any errors. There is also a `uro compare` command to
print out differences between Beancount and uromyces.

For more elaborate playing around it's probably best to write a Python script
that uses the `uromyces.load_file` function.
