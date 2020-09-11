.PHONY: all build test clean clippy

all: build doc
build:
	cargo build --features async
build_release:
	cargo build --release --features async
build_test:
	cargo test --no-run
doc:
	cargo test --doc --features async
	cargo doc --lib --release --no-deps --features async
test:
	cargo test -- --test-threads=1
	cargo test --features async -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy --tests --features async
