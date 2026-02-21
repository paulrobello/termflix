.PHONY: build release run test lint fmt typecheck checkall clean deploy ci

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

fmt-check:
	cargo fmt -- --check

typecheck:
	cargo check

checkall: fmt lint typecheck test build

clean:
	cargo clean

# Trigger CI workflow on GitHub
ci:
	gh workflow run ci.yml
	@echo "CI triggered — watch at https://github.com/paulrobello/termflix/actions"

# Trigger release + deploy workflow on GitHub
deploy:
	gh workflow run release.yml
	@echo "Deploy triggered — watch at https://github.com/paulrobello/termflix/actions"
