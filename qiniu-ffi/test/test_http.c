#include "unity.h"
#include "libqiniu_ng.h"

void test_qiniu_ng_http_headers_get_put(void) {
    qiniu_ng_http_headers_t headers = qiniu_ng_http_headers_new();
    qiniu_ng_http_headers_put(headers, "Content-Type", "application/json");
    qiniu_ng_http_headers_put(headers, "Content-Length", "1024");
    qiniu_ng_http_headers_put(headers, "Accept", "text/html");

    qiniu_ng_str_t value = qiniu_ng_http_headers_get(headers, "content-type");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(value), "application/json", "qiniu_ng_str_get_cstr() RETURNS WRONG VALUE");
    qiniu_ng_str_free(&value);

    value = qiniu_ng_http_headers_get(headers, "Content-length");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(value), "1024", "qiniu_ng_str_get_cstr() RETURNS WRONG VALUE");
    qiniu_ng_str_free(&value);

    value = qiniu_ng_http_headers_get(headers, "accept");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(value), "text/html", "qiniu_ng_str_get_cstr() RETURNS WRONG VALUE");
    qiniu_ng_str_free(&value);

    qiniu_ng_http_headers_free(&headers);
}
