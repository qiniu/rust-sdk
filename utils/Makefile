.PHONY: all build build_release build_test doc test doc_test clean clippy

all: build doc
build:
	cargo build
	cargo build --features='async_std_runtime'
	cargo build --features='tokio_runtime'
	cargo build --features='async_std_runtime macros'
	cargo build --features='tokio_runtime macros'
build_release:
	cargo build --release
	cargo build --release --features='async_std_runtime'
	cargo build --release --features='tokio_runtime'
	cargo build --release --features='async_std_runtime macros'
	cargo build --release --features='tokio_runtime macros'
build_test:
	cargo test --no-run
	cargo test --no-run --features='async_std_runtime'
	cargo test --no-run --features='tokio_runtime'
	cargo test --no-run --features='async_std_runtime macros'
	cargo test --no-run --features='tokio_runtime macros'
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc
test:
	cargo test -- --show-output
	cargo test --features='async_std_runtime' -- --show-output
	cargo test --features='tokio_runtime' -- --show-output
	cargo test --features='async_std_runtime macros' -- --show-output
	cargo test --features='tokio_runtime macros' -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='async_std_runtime' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='tokio_runtime' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='async_std_runtime macros' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='tokio_runtime macros' -- -D warnings --no-deps
