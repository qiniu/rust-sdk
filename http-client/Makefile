.PHONY: all build build_release build_test doc test clean clippy

all: build doc
build:
	cargo build --no-default-features --features ureq
	cargo build --no-default-features --features isahc
	cargo build --no-default-features --features reqwest
	cargo build --no-default-features --features 'async isahc'
	cargo build --no-default-features --features 'async reqwest'
	cargo build --no-default-features --features 'async c_ares trust_dns'
build_release:
	cargo build --no-default-features --release --features ureq
	cargo build --no-default-features --release --features isahc
	cargo build --no-default-features --release --features reqwest
	cargo build --no-default-features --release --features 'async isahc'
	cargo build --no-default-features --release --features 'async reqwest'
	cargo build --no-default-features --release --features 'async c_ares trust_dns'
build_test:
	cargo test --no-run
	cargo test --no-run --no-default-features --features isahc
	cargo test --no-run --no-default-features --features reqwest
	cargo test --no-run --no-default-features --features 'async isahc'
	cargo test --no-run --no-default-features --features 'async reqwest'
	cargo test --no-run --no-default-features --features 'async c_ares trust_dns'
doc:
	cargo test --doc --features 'async c_ares trust_dns'
	cargo +nightly doc --lib --release --all-features
test:
	cargo test --no-default-features --features ureq -- --test-threads=1 --show-output
	cargo test --no-default-features --features isahc -- --test-threads=1 --show-output
	cargo test --no-default-features --features reqwest -- --test-threads=1 --show-output
	cargo test --no-default-features --features 'async isahc' -- --test-threads=1 --show-output
	cargo test --no-default-features --features 'async reqwest' -- --test-threads=1 --show-output
	cargo test --no-default-features --features 'async c_ares trust_dns' -- --test-threads=1 --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- -D warnings --no-deps
