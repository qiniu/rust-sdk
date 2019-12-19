.PHONY: all build test clean clippy

all: build
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
test:
	cargo test -- --test-threads=1
clean:
	cargo clean
clippy:
	cargo clippy
