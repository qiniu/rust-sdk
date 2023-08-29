.PHONY: all build build_release build_test doc test doc_test clean clippy

all: build doc
build:
	cargo build
	cargo build --features='async-std-runtime'
	cargo build --features='tokio-runtime'
	cargo build --features='async-std-runtime macros'
	cargo build --features='tokio-runtime macros'
build_release:
	cargo build --release
	cargo build --release --features='async-std-runtime'
	cargo build --release --features='tokio-runtime'
	cargo build --release --features='async-std-runtime macros'
	cargo build --release --features='tokio-runtime macros'
build_test:
	cargo test --no-run
	cargo test --no-run --features='async-std-runtime'
	cargo test --no-run --features='tokio-runtime'
	cargo test --no-run --features='async-std-runtime macros'
	cargo test --no-run --features='tokio-runtime macros'
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc
test:
	cargo test -- --show-output
	cargo test --features='async-std-runtime' -- --show-output
	cargo test --features='tokio-runtime' -- --show-output
	cargo test --features='async-std-runtime macros' -- --show-output
	cargo test --features='tokio-runtime macros' -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='async-std-runtime' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='tokio-runtime' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='async-std-runtime macros' -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --features='tokio-runtime macros' -- -D warnings --no-deps
