.PHONY: all build build_release build_test doc test clean clippy

all: build doc
build:
	cargo build --no-default-features --features curl
	cargo build --no-default-features --features c_ares
	cargo build --no-default-features --features 'async c_ares'
	cargo build --no-default-features --features 'curl async'
build_release:
	cargo build --no-default-features --release --features curl
	cargo build --no-default-features --release --features c_ares
	cargo build --no-default-features --release --features 'async c_ares'
	cargo build --no-default-features --release --features 'curl async'
build_test:
	cargo test --no-run
	cargo test --no-run --no-default-features --features curl
	cargo test --no-run --no-default-features --features c_ares
	cargo test --no-run --no-default-features --features 'async c_ares'
	cargo test --no-run --no-default-features --features 'curl async'
doc:
	cargo test --doc --features 'async c_ares'
	cargo doc --lib --release --no-deps --features 'async c_ares'
test:
	cargo test --no-default-features -- --test-threads=1
	cargo test --no-default-features --features curl -- --test-threads=1
	cargo test --no-default-features --features c_ares -- --test-threads=1
	cargo test --no-default-features --features 'async c_ares' -- --test-threads=1
	cargo test --no-default-features --features 'curl async' -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy --features 'async c_ares'
