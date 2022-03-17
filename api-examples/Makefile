.PHONY: all build build_release build_test doc test clean clippy test-wasm

all: build doc
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
doc:
test:
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- -D warnings --no-deps
test-wasm:
