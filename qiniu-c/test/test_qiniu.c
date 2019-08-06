#include "unity.h"
#include "libqiniu_ng.h"
#include "string.h"

void write_str_to_file(const char* path, const char* content);

void test_qiniu_ng_etag_from_file(void) {
    char *etag = NULL;
    const char *path = "/tmp/1024k";
    write_str_to_file(path, "Hello world\n");
    TEST_ASSERT(qiniu_ng_etag_from_file(path, strlen(path), &etag) == 0);
    TEST_ASSERT(strcmp(etag, "FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d") == 0);
    free(etag);
}

void setUp(void) {

}

void tearDown(void) {

}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_qiniu_ng_etag_from_file);
    return UNITY_END();
}

void write_str_to_file(const char* path, const char* content) {
    FILE *fp = fopen(path, "w+");
    if (fp == NULL) {
        TEST_FAIL_MESSAGE("fopen() failed");
    }
    fprintf(fp, "%s", content);
    fclose(fp);
}
