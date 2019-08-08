.PHONY: all build test clean
CC=gcc
TARGET=build/test.out
UNITY_HOME=$(CURDIR)/test/unity/src
SRC_FILES=$(UNITY_HOME)/unity.c $(wildcard test/*.c)
INC_DIRS=-I$(CURDIR) -I$(UNITY_HOME)
STATIC_LIBRARY=$(CURDIR)/../target/debug/libqiniu_ng.a
DYNAMIC_LIBRARY=$(CURDIR)/../target/debug
SYMBOLS=-DTEST

all: build
build:
	cargo build
test: build
	mkdir -p build
	$(CC) -Wall -Wextra $(INC_DIRS) $(SYMBOLS) $(SRC_FILES) $(STATIC_LIBRARY) -o $(TARGET) -lm -lpthread -ldl
	./$(TARGET)
	rm ./$(TARGET)
	$(CC) -Wall -Wextra $(INC_DIRS) -L$(DYNAMIC_LIBRARY) $(SYMBOLS) $(SRC_FILES) -o $(TARGET) -lqiniu_ng -lm -lpthread -ldl
	LD_LIBRARY_PATH=$(DYNAMIC_LIBRARY) ./$(TARGET)
clean:
	rm -rf build
	cargo clean
