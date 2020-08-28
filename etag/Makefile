.PHONY: all build test clean clippy

all: build doc
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
doc:
	cargo test --doc
	cargo doc --lib --release --no-deps
test:
	cargo test -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy
