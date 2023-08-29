.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build
	cargo build --features async-std-runtime
	cargo build --features tokio-runtime
build_release:
	cargo build --release
	cargo build --release --features async-std-runtime
	cargo build --release --features tokio-runtime
build_test:
	cargo test --no-run
	cargo test --features async-std-runtime --no-run
	cargo test --features tokio-runtime --no-run
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features async-std-runtime
	cargo test --doc --features tokio-runtime
test:
	cargo test -- --show-output
	cargo test --features async-std-runtime -- --show-output
	cargo test --features tokio-runtime -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
