#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"
#include <string.h>

static qiniu_ng_credential_t get_credential(void) {
    return qiniu_ng_credential_new(QINIU_NG_CHARS("abcdefghklmnopq"), QINIU_NG_CHARS("1234567890"));
}

void test_qiniu_ng_credential_new(void) {
    qiniu_ng_credential_t credential = get_credential();
    qiniu_ng_str_t access_key, secret_key;

    access_key = qiniu_ng_credential_get_access_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(access_key),
        QINIU_NG_CHARS("abcdefghklmnopq"),
        "qiniu_ng_credential_get_access_key() returns unexpected value");

    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(secret_key),
        QINIU_NG_CHARS("1234567890"),
        "qiniu_ng_credential_get_secret_key() returns unexpected value");

    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_sign(void) {
    qiniu_ng_credential_t credential = get_credential();
    qiniu_ng_str_t signature;

    signature = qiniu_ng_credential_sign(credential, "hello", strlen("hello"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="),
        "qiniu_ng_credential_sign() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign(credential, "world", strlen("world"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="),
        "qiniu_ng_credential_sign() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign(credential, "-test", strlen("-test"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="),
        "qiniu_ng_credential_sign() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign(credential, "ba#a-", strlen("ba#a-"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:2d_Yr6H1GdTKg3RvMtpHOhi047M="),
        "qiniu_ng_credential_sign() returns unexpected value");
    qiniu_ng_str_free(&signature);

    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_sign_with_data(void) {
    qiniu_ng_credential_t credential = get_credential();
    qiniu_ng_str_t signature;

    signature = qiniu_ng_credential_sign_with_data(credential, "hello", strlen("hello"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8="),
        "qiniu_ng_credential_sign_with_data() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign_with_data(credential, "world", strlen("world"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ="),
        "qiniu_ng_credential_sign_with_data() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign_with_data(credential, "-test", strlen("-test"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q="),
        "qiniu_ng_credential_sign_with_data() returns unexpected value");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign_with_data(credential, "ba#a-", strlen("ba#a-"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(signature),
        QINIU_NG_CHARS("abcdefghklmnopq:kwzeJrFziPDMO4jv3DKVLDyqud0=:YmEjYS0="),
        "qiniu_ng_credential_sign_with_data() returns unexpected value");
    qiniu_ng_str_free(&signature);

    qiniu_ng_credential_free(&credential);
}
