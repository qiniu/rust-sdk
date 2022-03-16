.PHONY: all build build_release build_test doc test clean clippy test-wasm
SUBDIRS := utils credential etag upload-token http http-ureq http-isahc http-reqwest http-client api-generator apis api-examples objects-manager upload-manager

all:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir; \
	done
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
doc:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir doc; \
	done
test:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir test; \
	done
clean:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
clippy:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clippy; \
	done
test-wasm:
	set -e; \
		(cd etag && cargo wasi test -- --show-output); \
		(cd upload-token && cargo wasi test -- --show-output)
