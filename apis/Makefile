.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build
	cargo build --features async
build_release:
	cargo build --release
	cargo build --release --features async
build_test:
	cargo test --no-run
	cargo test --features async --no-run
doc:
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features async
test:
	cargo test --features async -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- --no-deps
