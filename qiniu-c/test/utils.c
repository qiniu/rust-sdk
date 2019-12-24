#include "unity.h"
#include "test.h"
#include <stdio.h>

void write_str_to_file(const char* path, const char* content) {
    FILE *fp = fopen(path, "w+");
    if (fp == NULL) {
        TEST_FAIL_MESSAGE("fopen() failed");
    }
    fprintf(fp, "%s", content);
    fclose(fp);
}

char* create_temp_file(size_t size) {
    const size_t FILE_PATH_LEN = 40;
    const size_t BUF_LEN = 4096;
    char *file_path = (char *) malloc(FILE_PATH_LEN);
    sprintf((char *) file_path, "/tmp/随机测试文件-%lu", (unsigned long) time(NULL));

    FILE *src = fopen("/dev/urandom", "r");
    TEST_ASSERT_NOT_NULL(src);

    FILE *dest = fopen(file_path, "w+");
    TEST_ASSERT_NOT_NULL(dest);

    size_t rest = size;
    char *buf = (char *) malloc(BUF_LEN);
    while (rest > 0) {
        size_t to_write = rest;
        if (to_write > BUF_LEN) {
            to_write = BUF_LEN;
        }
        size_t got = fread(buf, sizeof(char), to_write, src);
        TEST_ASSERT_GREATER_THAN(0, got);
        size_t written = fwrite(buf, sizeof(char), got, dest);
        TEST_ASSERT_EQUAL_INT(written, got);
        rest -= written;
    }

    free(buf);
    fclose(dest);
    fclose(src);

    return file_path;
}
