.PHONY: all build test clean clippy

all: build
build:
	cargo build
build_release:
	cargo build --release
build_test:
	cargo test --no-run
test: build
clean:
	cargo clean
clippy:
	cargo clippy
