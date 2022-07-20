.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build
	cargo build --features async
	cargo build --features async,native-tls
	cargo build --features async,rustls-tls
build_release:
	cargo build --release
	cargo build --release --features async
	cargo build --release --features async,native-tls
	cargo build --release --features async,rustls-tls
build_test:
	cargo test --no-run
	cargo test --features async --no-run
	cargo test --features async,native-tls --no-run
	cargo test --features async,rustls-tls --no-run
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features async
test:
	cargo test -- --show-output
	cargo test --features async -- --show-output
	cargo test --features async,native-tls -- --show-output
	cargo test --features async,rustls-tls -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
