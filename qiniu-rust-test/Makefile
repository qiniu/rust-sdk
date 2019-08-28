.PHONY: all build test test-libcurl clean

all: build
build:
	cargo build
test: test-libcurl
test-libcurl:
	cargo test --features use-libcurl
clean:
	cargo clean
