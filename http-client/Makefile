.PHONY: all build build_release build_test doc doc_test test clean clippy

all: build doc
build:
	cargo build --no-default-features --features ureq
	cargo build --no-default-features --features isahc
	cargo build --no-default-features --features reqwest
	cargo build --no-default-features --features 'async_std_runtime isahc'
	cargo build --no-default-features --features 'async_std_runtime c_ares trust_dns'
	cargo build --no-default-features --features 'tokio_runtime isahc'
	cargo build --no-default-features --features 'tokio_runtime reqwest'
	cargo build --no-default-features --features 'tokio_runtime c_ares trust_dns'
build_release:
	cargo build --no-default-features --release --features ureq
	cargo build --no-default-features --release --features isahc
	cargo build --no-default-features --release --features reqwest
	cargo build --no-default-features --release --features 'async_std_runtime isahc'
	cargo build --no-default-features --release --features 'async_std_runtime c_ares trust_dns'
	cargo build --no-default-features --release --features 'tokio_runtime isahc'
	cargo build --no-default-features --release --features 'tokio_runtime reqwest'
	cargo build --no-default-features --release --features 'tokio_runtime c_ares trust_dns'
build_test:
	cargo test --no-run --tests
	cargo test --no-run --tests --no-default-features --features isahc
	cargo test --no-run --tests --no-default-features --features reqwest
	cargo test --no-run --no-default-features --features 'async_std_runtime isahc'
	cargo test --no-run --no-default-features --features 'async_std_runtime c_ares trust_dns'
	cargo test --no-run --no-default-features --features 'tokio_runtime isahc'
	cargo test --no-run --no-default-features --features 'tokio_runtime reqwest'
	cargo test --no-run --no-default-features --features 'tokio_runtime c_ares trust_dns'
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	cargo test --doc --features 'async_std_runtime c_ares trust_dns ureq isahc reqwest'
	cargo test --doc --features 'tokio_runtime c_ares trust_dns ureq isahc reqwest'
test:
	cargo test --tests --no-default-features --features ureq -- --show-output
	cargo test --tests --no-default-features --features isahc -- --show-output
	cargo test --tests --no-default-features --features reqwest -- --show-output
	cargo test --no-default-features --features 'async_std_runtime isahc' -- --show-output
	cargo test --no-default-features --features 'async_std_runtime c_ares trust_dns' -- --show-output
	cargo test --no-default-features --features 'tokio_runtime isahc' -- --show-output
	cargo test --no-default-features --features 'tokio_runtime reqwest' -- --show-output
	cargo test --no-default-features --features 'tokio_runtime c_ares trust_dns' -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
