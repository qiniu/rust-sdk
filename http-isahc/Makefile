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
	cargo doc --lib --release --no-deps --features async
test:
	cargo test -- --test-threads=1 --nocapture
	cargo test --features async -- --test-threads=1 --nocapture
clean:
	cargo clean
clippy:
	cargo clippy --tests --features 'async'