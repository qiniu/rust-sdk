SUBDIRS := qiniu-rust qiniu-c

all: $(SUBDIRS)
$(SUBDIRS):
	$(MAKE) -C $@
build:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir build; \
	done
clean:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
test:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir test; \
	done

.PHONY: all build clean test $(SUBDIRS)
