all:

.PHONY: py
py:
	maturin build --release
	pip install target/wheels/*.whl --force-reinstall
	./run_python.py

.PHONY: lint
lint:
	cargo fmt
	cargo clippy

.PHONY: test
test:
	cargo test
	cargo doc

.PHONY: snapshots
snapshots:
	cargo insta test --delete-unreferenced-snapshots
	cargo insta review
