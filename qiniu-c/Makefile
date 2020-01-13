.PHONY: all build build_release build_test_via_static_link build_test_via_dynamic_link build_test test clean clippy
CC =
TARGET =
CFLAGS =
UNITY_HOME = $(CURDIR)/test/unity/src
SRC_FILES = $(UNITY_HOME)/unity.c $(wildcard test/*.c)
INC_DIRS =
STATIC_LIBRARY =
DYNAMIC_LIBRARY =
LDFLAGS =
SYMBOLS =

ifeq ($(OS),Windows_NT)
	CC := cl.exe
	TARGET := build/test.exe
	INC_DIRS := /I . /I $(UNITY_HOME)
	SYMBOLS := /D TEST /D UNITY_USE_FLUSH_STDOUT
	CFLAGS := /MD /W4 /utf-8
else
	CC := gcc
	TARGET := build/test.out
	INC_DIRS := -I$(CURDIR) -I$(UNITY_HOME)
	STATIC_LIBRARY := $(CURDIR)/../target/debug/libqiniu_ng_c.a
	DYNAMIC_LIBRARY := $(CURDIR)/../target/debug
	LDFLAGS := -lm -lpthread -ldl -lcurl
	SYMBOLS := -DTEST -DUNITY_USE_FLUSH_STDOUT
	CFLAGS := -O0 -Wall -Wextra

	ifeq ($(shell uname -s),Darwin)
		LDFLAGS += -framework Security -framework Foundation
	endif
endif

all: build
build:
	cargo build
	$(MAKE) libqiniu_ng.h
build_release:
	cargo build --release
	$(MAKE) libqiniu_ng.h
build_test_via_static_link: build
ifneq ($(OS),Windows_NT)
	mkdir -p build
	$(CC) $(CFLAGS) $(INC_DIRS) $(SYMBOLS) $(SRC_FILES) $(STATIC_LIBRARY) -o $(TARGET) $(LDFLAGS)
endif
build_test_via_dynamic_link: build
ifeq ($(OS),Windows_NT)
	if [ -d build ]; \
	then \
		rd /s /q build; \
	fi
	mkdir build
	$(CC) $(CFLAGS) $(SRC_FILES) $(INC_DIRS) $(SYMBOLS) /link /out:$(TARGET)
else
	mkdir -p build
	$(CC) $(CFLAGS) $(INC_DIRS) -L$(DYNAMIC_LIBRARY) $(SYMBOLS) $(SRC_FILES) -o $(TARGET) -lqiniu_ng_c $(LDFLAGS)
endif
build_test: build_test_via_static_link build_test_via_dynamic_link
libqiniu_ng.h: cbindgen.toml $(wildcard src/*.rs)
	cbindgen --config cbindgen.toml --crate qiniu-ng-c --output libqiniu_ng.h --quiet
test:
ifeq ($(OS),Windows_NT)
	$(MAKE) build_test_via_dynamic_link
	$(TARGET)
	rm $(TARGET)
else
	$(MAKE) build_test_via_static_link
	./$(TARGET)
	rm $(TARGET)
	$(MAKE) build_test_via_dynamic_link
	LD_LIBRARY_PATH=$(DYNAMIC_LIBRARY) ./$(TARGET)
endif
clean:
ifeq ($(OS),Windows_NT)
	if [ -d build ]; \
	then \
		rd /s /q build; \
	fi
else
	rm -rf build
endif
	cargo clean
clippy:
	cargo clippy
