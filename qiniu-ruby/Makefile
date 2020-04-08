BINDINGS = lib/qiniu_ng/bindings.rb
LIBQINIU_HEADER = ../qiniu-c/libqiniu_ng.h

all: build doc
build: $(BINDINGS) bundle
	bundle exec rake before_build build after_build
build_test: $(BINDINGS) bundle
	bundle exec rake build
	bundle exec rspec --dry-run
$(BINDINGS): bundle $(LIBQINIU_HEADER)
	bundle exec rake generate_bindings
$(LIBQINIU_HEADER):
	$(MAKE) -C ../qiniu-c build
doc: $(BINDINGS)
	bundle exec rake generate_doc
clean: bundle
	$(RM) $(BINDINGS)
	bundle exec rake clean
test: $(BINDINGS) bundle
	bundle exec rspec -f d --fail-fast
bundle:
	bundle install
.PHONY: all build clean test bundle
