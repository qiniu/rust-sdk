.PHONY: all build test clean clippy

all: build doc
build:
	cargo build --features curl
	cargo build --features async
	cargo build --features 'async c_ares'
build_release:
	cargo build --release --features curl
	cargo build --release --features async
	cargo build --release --features 'async c_ares'
build_test:
	cargo test --no-run
	cargo test --no-run --features curl
	cargo test --no-run --features async
	cargo test --no-run --features 'async c_ares'
doc:
	cargo test --doc --features 'async c_ares'
	cargo doc --lib --release --no-deps --features 'async c_ares'
test:
	cargo test -- --test-threads=1
	cargo test --features curl -- --test-threads=1
	cargo test --features async -- --test-threads=1
	cargo test --features 'async c_ares' -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy --features 'async c_ares'
