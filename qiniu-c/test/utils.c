#include "unity.h"
#include "test.h"
#include <stdio.h>

void write_str_to_file(const qiniu_ng_char_t* path, const char* content) {
    FILE *fp = OPEN_FILE_FOR_WRITING(path);
    if (fp == NULL) {
        TEST_FAIL_MESSAGE("fopen() failed");
    }
    fprintf(fp, "%s", content);
    fclose(fp);
}

qiniu_ng_char_t* create_temp_file(size_t size) {
    const size_t FILE_PATH_LEN = 256;
    const size_t BUF_LEN = 4096;
    qiniu_ng_char_t *file_path = (qiniu_ng_char_t *) malloc(FILE_PATH_LEN * sizeof(qiniu_ng_char_t));
#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_path, FILE_PATH_LEN, L"%ls\\随机测试文件-%lld", _wgetenv(L"TMP"), (long long) time(NULL));
#else
    snprintf((char *) file_path, FILE_PATH_LEN, "/tmp/随机测试文件-%lld", (long long) time(NULL));
#endif
    FILE *dest = OPEN_FILE_FOR_WRITING(file_path);
    TEST_ASSERT_NOT_NULL_MESSAGE(
        dest, "dest == null");
    size_t rest = size;
    char *buf = (char *) malloc(BUF_LEN);
    TEST_ASSERT_NOT_NULL_MESSAGE(
        buf, "buf != null");

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
        TEST_ASSERT_EQUAL_INT_MESSAGE(
            written, to_write,
            "written != to_write");
        rest -= written;
    }

    free(buf);
    fclose(dest);

    return file_path;
}
