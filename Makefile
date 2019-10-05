SUBDIRS := qiniu-rust qiniu-rust-http qiniu-rust-with-libcurl qiniu-rust-test qiniu-rust-test-utils qiniu-c

all: $(SUBDIRS)
$(SUBDIRS):
	$(MAKE) -C $@
build:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir build; \
	done
clean:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
test:
	set -e; \
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir test; \
	done

.PHONY: all build clean test $(SUBDIRS)
