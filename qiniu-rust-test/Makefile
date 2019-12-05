.PHONY: all build test test-libcurl clean clippy

all: build
build:
	cargo build
build_test:
	cargo test --no-run --features use-libcurl
test: test-libcurl
test-libcurl:
	cargo test --features use-libcurl
clean:
	cargo clean
clippy:
	cargo clippy
