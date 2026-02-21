.PHONY: build release run test lint fmt typecheck checkall clean

build:
	cargo build

release:
	cargo build --release

run:
	cargo run --release -- $(ARGS)

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

typecheck:
	cargo check

checkall: fmt lint typecheck test build

clean:
	cargo clean
