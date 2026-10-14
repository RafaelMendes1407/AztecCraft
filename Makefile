.PHONY: build test lint fmt check

build:
	cargo build --locked --quiet

test:
	cargo test --locked --quiet

lint:
	cargo fmt --all -- --check
	cargo clippy --locked --all-targets --quiet -- -D warnings

fmt:
	cargo fmt --all

check: lint test build
