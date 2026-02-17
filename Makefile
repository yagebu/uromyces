all: .venv

RUST_SOURCES := $(shell find src tree-sitter-beancount -type f)

# note: These commands are run in the venv with `uv run` to
#       avoid rebuilds: https://github.com/PyO3/pyo3/issues/1708
CARGO = uv run --no-project cargo

# Create and sync a dev environment.
.venv: uv.lock pyproject.toml
	uv sync
	touch -m .venv
# Rebuild Rust module.
python/uromyces/_uromyces.abi3.so: .venv $(RUST_SOURCES)
	uv sync --reinstall-package uromyces
dev: python/uromyces/_uromyces.abi3.so

# Run linters
lint: lint-rust lint-py
lint-py: dev
	pre-commit run -a
	uv run mypy python tests contrib
	uv run ty check python tests contrib
lint-rust: .venv
	$(CARGO) fmt
	$(CARGO) clippy

# Run Rust and Python tests
test: test-rust test-py
test-py: dev
	uv run pytest --cov=uromyces --cov-report=term-missing:skip-covered --cov-report=html --cov-fail-under=100
test-rust: .venv
	$(CARGO) test
test-rust-cov: .venv
	LLVM_COV=llvm-cov LLVM_PROFDATA=llvm-profdata $(CARGO) llvm-cov --html

# Generate Rust documentation
doc: .venv
	$(CARGO) doc --document-private-items

# Update lockfiles
update: .venv
	uv lock --upgrade
	pre-commit autoupdate
	$(CARGO) update
	$(CARGO) outdated

maturin-generate-ci: .venv
	uv run maturin generate-ci github --output=.github/workflows/maturin.yml --platform manylinux --platform windows --platform macos

# Update snapshot tests
insta: .venv
	-$(CARGO) insta test --unreferenced=delete
	$(CARGO) insta review

# Import Beancount booking_full_test DSL-based tests
import-booking-tests: .venv
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

.PHONY: clean dev doc insta lint lint-py lint-rust maturin-generate-ci test test-py test-rust test-rust-cov update
