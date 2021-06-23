.PHONY: all build build_release build_test doc test clean clippy
SUBDIRS := credential utils etag upload_token http http-client curl

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
