.PHONY: all build test clean
CC=gcc
TARGET=build/test.out
UNITY_HOME=$(CURDIR)/test/unity/src
SRC_FILES=$(UNITY_HOME)/unity.c $(wildcard test/*.c)
INC_DIRS=-I$(CURDIR) -I$(UNITY_HOME)
STATIC_LIBRARY=$(CURDIR)/../target/debug/libqiniu_ng_c.a
DYNAMIC_LIBRARY=$(CURDIR)/../target/debug
LDFLAGS=-lm -lpthread -ldl -lcurl
SYMBOLS=-DTEST

ifeq ($(OS),Windows_NT)
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S),Darwin)
		LDFLAGS += -framework Security -framework Foundation
	endif
endif

all: build
build:
	cargo build
test: build
	mkdir -p build
	$(CC) -Wall -Wextra $(INC_DIRS) $(SYMBOLS) $(SRC_FILES) $(STATIC_LIBRARY) -o $(TARGET) $(LDFLAGS)
	./$(TARGET)
	rm ./$(TARGET)
	$(CC) -Wall -Wextra $(INC_DIRS) -L$(DYNAMIC_LIBRARY) $(SYMBOLS) $(SRC_FILES) -o $(TARGET) -lqiniu_ng_c $(LDFLAGS)
	LD_LIBRARY_PATH=$(DYNAMIC_LIBRARY) ./$(TARGET)
clean:
	rm -rf build
	cargo clean
