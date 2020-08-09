#include "unity.h"
#include "libqiniu_ng.h"

void test_qiniu_ng_str_new(void) {
    qiniu_ng_str_t str = qiniu_ng_str_new(NULL);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_str_is_null(str), "qiniu_ng_str_is_null() RETURNS FALSE");
    qiniu_ng_str_free(&str);

    str = qiniu_ng_str_new("hello world");
    TEST_ASSERT_FALSE_MESSAGE(qiniu_ng_str_is_null(str), "qiniu_ng_str_is_null() RETURNS TRUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(str), "hello world", "qiniu_ng_str_get_cstr() RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&str);
}

void test_qiniu_ng_str_push_cstr(void) {
    qiniu_ng_str_t str = qiniu_ng_str_new(NULL);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_str_is_null(str), "qiniu_ng_str_is_null() RETURNS FALSE");
    qiniu_ng_str_push_cstr(&str, "hello world");
    TEST_ASSERT_FALSE_MESSAGE(qiniu_ng_str_is_null(str), "qiniu_ng_str_is_null() RETURNS TRUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(str), "hello world", "qiniu_ng_str_get_cstr() RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_push_cstr(&str, "!!!");
    TEST_ASSERT_FALSE_MESSAGE(qiniu_ng_str_is_null(str), "qiniu_ng_str_is_null() RETURNS TRUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(str), "hello world!!!", "qiniu_ng_str_get_cstr() RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&str);
}
