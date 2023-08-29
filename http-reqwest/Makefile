.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build
	cargo build --features tokio-runtime
	cargo build --features tokio-runtime,native-tls
	cargo build --features tokio-runtime,rustls-tls
build_release:
	cargo build --release
	cargo build --release --features tokio-runtime
	cargo build --release --features tokio-runtime,native-tls
	cargo build --release --features tokio-runtime,rustls-tls
build_test:
	cargo test --no-run
	cargo test --features tokio-runtime --no-run
	cargo test --features tokio-runtime,native-tls --no-run
	cargo test --features tokio-runtime,rustls-tls --no-run
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features tokio-runtime
test:
	cargo test -- --show-output
	cargo test --features tokio-runtime -- --show-output
	cargo test --features tokio-runtime,native-tls -- --show-output
	cargo test --features tokio-runtime,rustls-tls -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
