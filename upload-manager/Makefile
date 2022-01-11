.PHONY: all build build_release build_test doc test clean clippy

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
	cargo test --doc --features async
	cargo +nightly doc --lib --release --all-features
test:
	cargo test -- --show-output
	cargo test --features async -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- --no-deps