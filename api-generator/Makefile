.PHONY: all build build_release build_test doc doc_test test clean clippy reset_apis

all: build doc
build: reset_apis
	cargo run
build_release: build
build_test:
	cargo test --no-run
doc: build
	make -C ../apis doc
doc_test: build
	make -C ../apis doc_test
test:
	cargo test -- --show-output
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- --no-deps
reset_apis:
	rm -rf ../apis/src
	mkdir ../apis/src
	touch ../apis/src/lib.rs
