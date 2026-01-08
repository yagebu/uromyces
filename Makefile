all: .venv

RUST_SOURCES := $(shell find src tree-sitter-beancount -type f)

# note: These commands are run in the venv with `uv run` to
#       avoid rebuilds: https://github.com/PyO3/pyo3/issues/1708
CARGO = uv run cargo

# Create and sync a dev environment, making sure to recompile the Rust module.
.venv: uv.lock pyproject.toml $(RUST_SOURCES)
	uv sync --reinstall-package uromyces
	touch -m .venv
# Rebuild Rust module (should normally not be needed).
dev:
	uv sync --reinstall-package uromyces

# Run linters
lint: .venv
	$(CARGO) fmt
	$(CARGO) clippy
	pre-commit run -a
	uv run mypy uromyces tests contrib
	uv run ty check uromyces tests contrib

# Run Rust and Python tests
test: test-rust test-py
test-py: .venv
	uv run pytest --cov=uromyces --cov-report=term-missing:skip-covered --cov-report=html
test-rust:
	$(CARGO) test

# Generate Rust documentation
doc: .venv
	$(CARGO) doc --document-private-items

# Update lockfiles
update: .venv
	uv lock --upgrade
	pre-commit autoupdate
	$(CARGO) update
	$(CARGO) outdated
	# uv run maturin generate-ci github > .github/workflows/maturin.yml

# Update snapshot tests
insta: .venv
	-$(CARGO) insta test --unreferenced=delete
	$(CARGO) insta review

# Import Beancount booking_full_test DSL-based tests
import-booking-tests:
	rm -f src/booking/test_inputs/*.beancount
	uv run contrib/scripts.py import-booking-tests

clean:
	rm -rf .*cache
	rm -rf .coverage
	rm -rf .venv
	rm -rf dist
	rm -rf htmlcov
	rm -rf target
	find . -type f -name '*.so' -delete
	find . -type f -name '*.py[c0]' -delete
	find . -type d -name "__pycache__" -delete

.PHONY: clean dev doc insta lint test test-py test-rust update
