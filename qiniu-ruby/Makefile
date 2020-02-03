all: build
build: lib/qiniu_ng/bindings.rb bundle
	bundle exec rake build
lib/qiniu_ng/bindings.rb: bundle ../qiniu-c/libqiniu_ng.h
	bundle exec rake generate_bindings
../qiniu-c/libqiniu_ng.h:
	$(MAKE) -C ../qiniu-c build
clean: bundle
	$(RM) lib/qiniu_ng/bindings.rb
	bundle exec rake clean
test: bundle
	bundle exec rspec
bundle:
	bundle install
.PHONY: all build clean test bundle
