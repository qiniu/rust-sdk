.PHONY: all build test clean clippy

all: build doc
build:
	cargo build
build_test:
	cargo test --no-run
doc:
	cargo test --doc
	cargo doc --lib --release --no-deps
test:
	cargo test
clean:
	cargo clean
clippy:
	cargo clippy
