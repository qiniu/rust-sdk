.PHONY: all build build_release build_test doc test clean clippy

all: build doc
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
doc:
	cargo test --doc
	cargo +nightly doc --lib --release --all-features
test:
	cargo test -- --test-threads=1 --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- -D warnings --no-deps
