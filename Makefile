all: .venv

# note: It's best to activate the venv before running the following
#       targets to avoid rebuilds: https://github.com/PyO3/pyo3/issues/1708

# Create and sync a dev environment.
.venv: uv.lock pyproject.toml
	uv sync
	touch -m .venv

# Compile Rust extension module
dev: .venv
	uv run maturin develop --uv --skip-install --release

# Run linters
lint: .venv
	cargo fmt
	cargo clippy
	pre-commit run -a
	uv run mypy uromyces tests contrib
	uv run ty check uromyces tests contrib

# Run Rust and Python tests
test: .venv
	cargo test
	uv run pytest

# Generate Rust documentation
doc: .venv
	cargo doc --document-private-items

# Update lockfiles
update: .venv
	uv lock --upgrade
	pre-commit autoupdate
	cargo update
	cargo outdated
	# uv run maturin generate-ci github > .github/workflows/maturin.yml

# Update snapshot tests
insta: .venv
	-cargo insta test --unreferenced=delete
	cargo insta review

# Import Beancount booking_full_test DSL-based tests
import-booking-tests:
	rm -f src/booking/test_inputs/*.beancount
	uv run contrib/scripts.py import-booking-tests

clean:
	rm -rf .*cache
	rm -rf .venv
	rm -rf target
	find . -type f -name '*.so' -delete
	find . -type f -name '*.py[c0]' -delete
	find . -type d -name "__pycache__" -delete

.PHONY: clean dev doc insta lint test update
