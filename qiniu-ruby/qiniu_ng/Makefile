all: build
build: lib/qiniu_ng/bindings.rb
	rake build
lib/qiniu_ng/bindings.rb:
	rake generate_bindings
clean:
	$(RM) lib/qiniu_ng/bindings.rb
	rake clean
test:
	# TODO
.PHONY: all build clean test
