#include "unity.h"
#include "libqiniu_ng.h"
#include "string.h"
#include "errno.h"
#include "test.h"

void test_qiniu_ng_etag_from_file_path(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const char *path = "/tmp/1024k";
    write_str_to_file(path, "Hello world\n");
    TEST_ASSERT_TRUE(qiniu_ng_etag_from_file_path(path, strlen(path), (char *) &etag, NULL));
    TEST_ASSERT_EQUAL_INT(strcmp((const char *) &etag, "FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d"), 0);
}

void test_qiniu_ng_etag_from_buffer(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const char *buf = "Hello world\n";
    qiniu_ng_etag_from_buffer(buf, strlen(buf), (char *) &etag);
    TEST_ASSERT_EQUAL_INT(strcmp((const char *) &etag, "FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d"), 0);
}

void test_qiniu_ng_etag_from_unexisted_file_path(void) {
    const char *path = "/not_existed";
    qiniu_ng_err err;
    int error_code;
    const char *error_description;
    TEST_ASSERT_FALSE(qiniu_ng_etag_from_file_path(path, strlen(path), NULL, &err));
    TEST_ASSERT_TRUE(qiniu_ng_err_os_error_extract(&err, &error_code, &error_description));
    TEST_ASSERT_EQUAL_INT(error_code, ENOENT);
    TEST_ASSERT_EQUAL_INT(strcmp(error_description, "No such file or directory"), 0);

    TEST_ASSERT_TRUE(qiniu_ng_err_os_error_extract(&err, &error_code, NULL));
    TEST_ASSERT_EQUAL_INT(error_code, ENOENT);

    TEST_ASSERT_TRUE(qiniu_ng_err_os_error_extract(&err, NULL, &error_description));
    TEST_ASSERT_EQUAL_INT(strcmp(error_description, "No such file or directory"), 0);

    TEST_ASSERT_TRUE(qiniu_ng_err_os_error_extract(&err, NULL, NULL));
}

void test_qiniu_ng_etag_from_large_buffer(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const char *buf = "Hello world\n";

    qiniu_ng_etag_t qiniu_ng_etag = qiniu_ng_etag_new();
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_result(qiniu_ng_etag, (char *) &etag);
    qiniu_ng_etag_reset(qiniu_ng_etag);

    TEST_ASSERT_EQUAL_INT(strcmp((const char *) &etag, "FgAgNanfbszl6CSk8MEyKDDXvpgG"), 0);

    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_result(qiniu_ng_etag, (char *) &etag);
    qiniu_ng_etag_free(qiniu_ng_etag);

    TEST_ASSERT_EQUAL_INT(strcmp((const char *) &etag, "FhV9_jRUUi8lQ9eL_AbKIZj5pWXx"), 0);
}
