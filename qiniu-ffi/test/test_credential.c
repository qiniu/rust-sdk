#include <string.h>
#include "unity.h"
#include "libqiniu_ng.h"

void test_qiniu_ng_credential_get(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");
    qiniu_ng_str_t access_key = qiniu_ng_credential_get_access_key(credential);
    qiniu_ng_str_t secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_sign(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");

    qiniu_ng_str_t signature = qiniu_ng_credential_sign(credential, "hello", strlen("hello"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(signature), "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=", "qiniu_ng_str_get_cstr() RETURNS WRONG RESULT");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign(credential, "world", strlen("world"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(signature), "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=", "qiniu_ng_str_get_cstr() RETURNS WRONG RESULT");
    qiniu_ng_str_free(&signature);

    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_sign_with_data(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");

    qiniu_ng_str_t signature = qiniu_ng_credential_sign_with_data(credential, "hello", strlen("hello"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(signature), "abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8=", "qiniu_ng_str_get_cstr() RETURNS WRONG RESULT");
    qiniu_ng_str_free(&signature);

    signature = qiniu_ng_credential_sign_with_data(credential, "world", strlen("world"));
    TEST_ASSERT_EQUAL_STRING_MESSAGE(qiniu_ng_str_get_cstr(signature), "abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ=", "qiniu_ng_str_get_cstr() RETURNS WRONG RESULT");
    qiniu_ng_str_free(&signature);

    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_authorization_v1(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");

    qiniu_ng_str_t authorization;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_authorization_v1_for_request(
            credential,
            "http://upload.qiniup.com/",
            "",
            "{\"name\":\"test\"}",
            strlen("{\"name\":\"test\"}"
        ), &authorization, NULL),
    "qiniu_ng_credential_authorization_v1_for_request() RETURNS FALSE");
    qiniu_ng_str_t signature = qiniu_ng_credential_sign(credential, "/\n", strlen("/\n"));
    qiniu_ng_str_t expected = qiniu_ng_str_new("QBox ");
    qiniu_ng_str_push_cstr(&expected, qiniu_ng_str_get_cstr(signature));
    qiniu_ng_str_free(&signature);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(authorization),
        qiniu_ng_str_get_cstr(expected),
        "authorization != expected");
    qiniu_ng_str_free(&authorization);
    qiniu_ng_str_free(&expected);
    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_authorization_v2(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");

    qiniu_ng_http_headers_t headers = qiniu_ng_http_headers_new();
    qiniu_ng_http_headers_put(headers, "Content-Type", "application/json");
    qiniu_ng_http_headers_put(headers, "X-Qbox-Meta", "value");
    qiniu_ng_http_headers_put(headers, "X-Qiniu-Cxxxx", "valuec");
    qiniu_ng_http_headers_put(headers, "X-Qiniu-Bxxxx", "valueb");
    qiniu_ng_http_headers_put(headers, "X-Qiniu-axxxx", "valuea");
    qiniu_ng_http_headers_put(headers, "X-Qiniu-e", "value");
    qiniu_ng_http_headers_put(headers, "X-Qiniu-", "value");
    qiniu_ng_http_headers_put(headers, "X-Qiniu", "value");

    qiniu_ng_str_t authorization;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_authorization_v2_for_request(
            credential,
            "http://upload.qiniup.com/",
            qiniu_ng_http_method_get,
            headers,
            "{\"name\":\"test\"}",
            strlen("{\"name\":\"test\"}"
        ), &authorization, NULL),
    "qiniu_ng_credential_authorization_v2_for_request() RETURNS FALSE");

    char signed_body[257];
    snprintf(signed_body, sizeof(signed_body), "%s%s%s%s%s%s%s%s",
        "GET /\n",
        "Host: upload.qiniup.com\n",
        "Content-Type: application/json\n",
        "X-Qiniu-Axxxx: valuea\n",
        "X-Qiniu-Bxxxx: valueb\n",
        "X-Qiniu-Cxxxx: valuec\n",
        "X-Qiniu-E: value\n\n",
        "{\"name\":\"test\"}");
    qiniu_ng_str_t signature = qiniu_ng_credential_sign(credential, signed_body, strlen(signed_body));
    qiniu_ng_str_t expected = qiniu_ng_str_new("Qiniu ");
    qiniu_ng_str_push_cstr(&expected, qiniu_ng_str_get_cstr(signature));
    qiniu_ng_str_free(&signature);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(authorization),
        qiniu_ng_str_get_cstr(expected),
        "authorization != expected");
    qiniu_ng_str_free(&expected);
    qiniu_ng_str_free(&authorization);
    qiniu_ng_http_headers_free(&headers);
    qiniu_ng_credential_free(&credential);
}

void test_qiniu_ng_credential_sign_download_url(void) {
    qiniu_ng_credential_t credential = qiniu_ng_credential_new("abcdefghklmnopq", "1234567890");

    qiniu_ng_str_t signed_url;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_sign_download_url(credential, "http://www.qiniu.com/?go=1", 1234567890 + 3600, &signed_url, NULL),
        "qiniu_ng_credential_sign_download_url() RETURNS FALSE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(signed_url),
        "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq%3AKjQtlGAkEOhSwtFjJfYtYa2-reE%3D",
        "qiniu_ng_str_get_cstr() RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&signed_url);
    qiniu_ng_credential_free(&credential);
}
