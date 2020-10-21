.PHONY: all build test clean clippy

all: build doc
build:
	cargo build --features 'async c_ares'
	cargo build --features curl
build_release:
	cargo build --release --features 'async c_ares'
	cargo build --release --features curl
build_test:
	cargo test --no-run
doc:
	cargo test --doc --features 'async c_ares'
	cargo doc --lib --release --no-deps --features 'async c_ares'
test:
	cargo test -- --test-threads=1
	cargo test --features 'async c_ares' -- --test-threads=1
	cargo test --features curl -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy --features 'async c_ares'
