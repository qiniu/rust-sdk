.PHONY: all build build_release build_test doc test clean clippy reset_apis

all: build doc
build: reset_apis
	cargo run
build_release: build
build_test:
	cargo test --no-run
doc:
test:
	cargo test -- --test-threads=1 --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --tests --all-features -- --no-deps
reset_apis:
	rm -rf ../apis/src
	mkdir ../apis/src
	touch ../apis/src/lib.rs
