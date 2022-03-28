.PHONY: all build build_release build_test doc test clean clippy

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
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
