check:
	cargo check
	cargo fmt --check
	cargo clippy

fmt:
	cargo fmt

.PHONY: check fmt
