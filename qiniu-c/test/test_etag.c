#include "unity.h"
#include "libqiniu_ng.h"
#include "string.h"
#include "errno.h"
#include "test.h"

void test_qiniu_ng_etag_from_file_path(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const size_t PATH_LEN = 256;
    qiniu_ng_char_t* path = (qiniu_ng_char_t *) malloc((PATH_LEN + 1) * sizeof(qiniu_ng_char_t));
    memset(path, 0, (PATH_LEN + 1) * sizeof(qiniu_ng_char_t));

#if defined(_WIN32) || defined(WIN32)
    swprintf(path, PATH_LEN, L"%s/1024字节", _wgetenv(L"TMP"));
#else
    strncpy(path, "/tmp/1024字节", PATH_LEN);
#endif
    write_str_to_file(path, "Hello world\n");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_etag_from_file_path(path, (char *) &etag, NULL),
        "qiniu_ng_etag_from_file_path() failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        (const char *) &etag, "FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d",
        "etag was wrong");
    free(path);
}

void test_qiniu_ng_etag_from_data(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const char *buf = "Hello world\n";
    qiniu_ng_etag_from_data((void *) buf, strlen(buf), (char *) &etag);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        (const char *) &etag, "FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d",
        "etag was wrong");
}

void test_qiniu_ng_etag_from_unexisted_file_path(void) {
    const qiniu_ng_char_t *path = QINIU_NG_CHARS("/不存在的文件");
    qiniu_ng_err_t err;
    int32_t os_err_code;
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_etag_from_file_path(path, NULL, &err),
        "qiniu_ng_etag_from_file_path() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &os_err_code),
        "qiniu_ng_err_os_error_extract() failed");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        os_err_code, ENOENT,
        "os_err_code != ENOENT");

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &os_err_code),
        "qiniu_ng_err_os_error_extract() returns unexpected value");
}

void test_qiniu_ng_etag_from_large_data(void) {
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));

    const char *buf = "Hello world\n";

    qiniu_ng_etag_t qiniu_ng_etag = qiniu_ng_etag_new();
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_result(qiniu_ng_etag, (char *) &etag);

    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        (const char *) &etag, "FgAgNanfbszl6CSk8MEyKDDXvpgG",
        "etag was wrong");

    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_update(qiniu_ng_etag, (void *) buf, strlen(buf));
    qiniu_ng_etag_result(qiniu_ng_etag, (char *) &etag);
    qiniu_ng_etag_free(&qiniu_ng_etag);

    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        (const char *) &etag, "FhV9_jRUUi8lQ9eL_AbKIZj5pWXx",
        "etag was wrong");
}
