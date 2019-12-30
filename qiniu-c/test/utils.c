#include "unity.h"
#include "test.h"
#include <stdio.h>

void write_str_to_file(const qiniu_ng_char_t* path, const char* content) {
#if defined(_WIN32) || defined(WIN32)
    FILE *fp = _wfopen(path, L"wb+");
#else
    FILE *fp = fopen(path, "w+");
#endif
    if (fp == NULL) {
        TEST_FAIL_MESSAGE("fopen() failed");
    }
    fprintf(fp, "%s", content);
    fclose(fp);
}

qiniu_ng_char_t* create_temp_file(size_t size) {
    const size_t FILE_PATH_LEN = 256;
    const size_t BUF_LEN = 4096;
#if defined(_WIN32) || defined(WIN32)
    wchar_t *file_path = (wchar_t *) malloc(FILE_PATH_LEN * sizeof(wchar_t));
    swprintf((wchar_t *) file_path, FILE_PATH_LEN, L"%ls\\随机测试文件-%lu", _wgetenv(L"TMP"), (unsigned long) time(NULL));

    FILE *dest = _wfopen(file_path, L"wb+");
    TEST_ASSERT_NOT_NULL(dest);
#else
    char *file_path = (char *) malloc(FILE_PATH_LEN * sizeof(char));
    sprintf((char *) file_path, "/tmp/随机测试文件-%lu", (unsigned long) time(NULL));

    FILE *dest = fopen(file_path, "w+");
    TEST_ASSERT_NOT_NULL(dest);
#endif
    size_t rest = size;
    char *buf = (char *) malloc(BUF_LEN);
    TEST_ASSERT_NOT_NULL(buf);

    srand(time(NULL));
    while (rest > 0) {
        size_t to_write = rest;
        if (to_write > BUF_LEN) {
            to_write = BUF_LEN;
        }
        for (size_t i = 0; i < to_write; i++) {
            buf[i] = (char) rand();
        }
        size_t written = fwrite(buf, sizeof(char), to_write, dest);
        TEST_ASSERT_EQUAL_INT(written, to_write);
        rest -= written;
    }

    free(buf);
    fclose(dest);

    return file_path;
}
