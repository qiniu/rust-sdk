.PHONY: all build build_release build_test doc doc_test test clean clippy publish
SUBDIRS := utils credential etag upload-token http http-ureq http-isahc http-reqwest http-client api-generator apis sdk-examples objects-manager upload-manager download-manager sdk

all: build doc
build:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir build; \
	done
build_release:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir build_release; \
	done
build_test:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir build_test; \
	done
doc: doc_test
	cargo +nightly doc --lib --release --all-features
doc_test:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir doc_test; \
	done
test:
	set -e; \
	for dir in $(SUBDIRS); do \
		if [ -n "${CLEAN_BEFORE_EACH}" ]; then \
			$(MAKE) clean; \
		fi; \
		$(MAKE) -C $$dir test; \
	done
clean:
	cargo clean
clippy:
	cargo +nightly clippy --examples -- -D warnings --no-deps
	cargo +nightly clippy --examples --tests --all-features -- -D warnings --no-deps
publish:
	set -e; \
	for i in utils/ etag/ credential/ http/ upload-token/ http-isahc/ http-reqwest/ http-ureq/ http-client/ apis/ objects-manager/ upload-manager/ download-manager sdk/ ; do \
		(cd $$i && cargo publish && sleep 300) || break ; \
	done
