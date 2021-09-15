.PHONY: all build build_release build_test doc test clean clippy

all: build doc
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
	cargo test --features async --no-run
doc:
	cargo test --doc --features async
	cargo +nightly doc --lib --release --no-deps --all-features
test:
	cargo test -- --test-threads=1
	cargo test --features async -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- -D warnings --no-deps
