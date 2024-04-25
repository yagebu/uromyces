all: lint test

.PHONY: lint
lint:
	cargo fmt
	cargo clippy
	pre-commit run -a

.PHONY: test
test:
	cargo test
	cargo doc --document-private-items
	tox

.PHONY: update
update:
	pre-commit autoupdate
	cargo update
	cargo outdated

.PHONY: insta
insta:
	-cargo insta test --unreferenced=delete
	cargo insta review
