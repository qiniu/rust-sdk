#include <string.h>
#include <stdio.h>
#include "unity.h"
#include "libqiniu_ng.h"

static FILE* make_fake_file(uint size);

void test_qiniu_ng_etag_v1(void) {
    char etag_buf[29];
    memset(etag_buf, 0, 29);
    qiniu_ng_etag_t etag = qiniu_ng_etag_new(1);
    qiniu_ng_etag_result(etag, etag_buf);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(etag_buf, "Fto5o-5ea0sNMlW_75VgGJCv2AcJ",
        "qiniu_ng_etag_result() returns expected result");

    qiniu_ng_etag_update(etag, "etag", strlen("etag"));
    qiniu_ng_etag_result(etag, etag_buf);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(etag_buf, "FpLiADEaVoALPkdb8tJEJyRTXoe_",
        "qiniu_ng_etag_result() returns expected result");
    qiniu_ng_etag_free(&etag);
}

void test_qiniu_ng_etag_v2(void) {
    char etag_buf[29];
    memset(etag_buf, 0, 29);
    qiniu_ng_etag_t etag = qiniu_ng_etag_new(2);
    qiniu_ng_etag_update(etag, "hello", strlen("hello"));
    qiniu_ng_etag_update(etag, "world", strlen("world"));
    qiniu_ng_etag_result(etag, etag_buf);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(etag_buf, "ns56DcSIfBFUENXjdhsJTIvl3Rcu",
        "qiniu_ng_etag_result() returns expected result");
    qiniu_ng_etag_free(&etag);
}

void test_qiniu_ng_etag_from_file(void) {
    char etag_buf[29];
    memset(etag_buf, 0, 29);

    FILE* tempfile = make_fake_file(1 << 20);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_etag_v1_of_file(tempfile, etag_buf, NULL), "qiniu_ng_etag_v1_of_file() returns false");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(etag_buf, "Foyl8onxBLWeRLL5oItRJphv6i4b", "etag_buf is not expected");
    fclose(tempfile);

    tempfile = make_fake_file(9 << 20);

    unsigned long parts[] = {1 << 22, 1 << 22, 1 << 20};
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_etag_v2_of_file(tempfile, parts, 3, etag_buf, NULL), "qiniu_ng_etag_v2_of_file() returns false");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(etag_buf, "ljgVjMtyMsOgIySv79U8Qz4TrUO4", "etag_buf is not expected");
    fclose(tempfile);
}

static FILE* make_fake_file(uint size) {
    const int BUF_LEN = 4096;
    char buf[BUF_LEN];
    memset(buf, 'b', BUF_LEN);
    buf[0] = 'A';
    buf[BUF_LEN - 2] = '\r';
    buf[BUF_LEN - 1] = '\n';

    FILE* tempfile = tmpfile();
    TEST_ASSERT_NOT_NULL_MESSAGE(tempfile, "tmpfile() returns NULL");

    uint rest = size;
    while (rest > 0) {
        uint add_size = rest;
        if (add_size > BUF_LEN) {
            add_size = BUF_LEN;
        }
        TEST_ASSERT_EQUAL_INT_MESSAGE(fwrite(buf, sizeof(char), add_size, tempfile), add_size, "fwrite() returns unexpected value");
        TEST_ASSERT_EQUAL_INT_MESSAGE(ferror(tempfile), 0, "ferror() returns non-zero");
        rest -= add_size;
    }

    TEST_ASSERT_EQUAL_INT_MESSAGE(fseek(tempfile, 0, 0), 0, "fseek() returns non-zero");
    return tempfile;
}
