.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build
	cargo build --features async_std_runtime
	cargo build --features tokio_runtime
build_release:
	cargo build --release
	cargo build --release --features async_std_runtime
	cargo build --release --features tokio_runtime
build_test:
	cargo test --no-run
	cargo test --features async_std_runtime --no-run
	cargo test --features tokio_runtime --no-run
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features async_std_runtime
	cargo test --doc --features tokio_runtime
test:
	cargo test --features async_std_runtime -- --show-output
	cargo test --features tokio_runtime -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples --tests -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
