SUBDIRS := qiniu-rust qiniu-rust-http qiniu-rust-with-libcurl qiniu-rust-test qiniu-rust-test-utils qiniu-c qiniu-c-translator
OTHER_LANG_DIRS := qiniu-ruby

all: $(SUBDIRS) $(OTHER_LANG_DIRS)
$(SUBDIRS) $(OTHER_LANG_DIRS):
	$(MAKE) -C $@
build:
	set -e; \
	for dir in $(SUBDIRS) $(OTHER_LANG_DIRS); do \
		$(MAKE) -C $$dir build; \
	done
build_release:
	set -e; \
	for dir in qiniu-rust qiniu-c; do \
		$(MAKE) -C $$dir build_release; \
	done
build_test:
	set -e; \
	for dir in $(SUBDIRS) $(OTHER_LANG_DIRS); do \
		$(MAKE) -C $$dir build_test; \
	done
doc:
	set -e; \
	for dir in qiniu-rust qiniu-c; do \
		$(MAKE) -C $$dir doc; \
	done
clean:
	set -e; \
	for dir in $(SUBDIRS) $(OTHER_LANG_DIRS); do \
		$(MAKE) -C $$dir clean; \
	done
test:
	set -e; \
	for dir in $(SUBDIRS) $(OTHER_LANG_DIRS); do \
		$(MAKE) -C $$dir test; \
	done
clippy:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clippy; \
	done
publish:
	set -e; \
	for dir in qiniu-rust-http qiniu-rust-with-libcurl qiniu-rust-test-utils qiniu-rust; do \
		(cd $$dir && cargo publish); \
	done

.PHONY: all build clean doc test $(SUBDIRS) $(OTHER_LANG_DIRS)
