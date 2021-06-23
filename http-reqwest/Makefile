.PHONY: all build build_release build_test doc test clean clippy

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
doc:
	cargo test --doc --features async
	cargo doc --lib --release --no-deps --features async,native-tls,rustls-tls
test:
	cargo test -- --test-threads=1 --nocapture
	cargo test --features async -- --test-threads=1 --nocapture
	cargo test --features async,native-tls -- --test-threads=1 --nocapture
	cargo test --features async,rustls-tls -- --test-threads=1 --nocapture
clean:
	cargo clean
clippy:
	cargo clippy --tests --features 'async'
