#include <stdlib.h>
#include "unity.h"
#include "libqiniu_ng.h"

void test_qiniu_ng_credential_provider_static(void) {
    qiniu_ng_credential_provider_t credential_provider = qiniu_ng_credential_provider_new_static("abcdefghklmnopq", "1234567890");

    qiniu_ng_credential_t credential;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(credential_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
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
    qiniu_ng_credential_provider_free(&credential_provider);
}

void test_qiniu_ng_credential_provider_global(void) {
    qiniu_ng_credential_provider_t credential_provider = qiniu_ng_credential_provider_new_global();
    qiniu_ng_credential_provider_global_setup("abcdefghklmnopq-1", "1234567890-1");

    qiniu_ng_credential_t credential;
    qiniu_ng_str_t access_key, secret_key;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(credential_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-1",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-1",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_global_setup("abcdefghklmnopq-2", "1234567890-2");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(credential_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-2",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-2",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_global_clear();
    qiniu_ng_credential_provider_free(&credential_provider);
}

void test_qiniu_ng_credential_provider_env(void) {
    qiniu_ng_credential_provider_t credential_provider = qiniu_ng_credential_provider_new_env();

    putenv("QINIU_ACCESS_KEY=abcdefghklmnopq-3");
    putenv("QINIU_SECRET_KEY=1234567890-3");
    qiniu_ng_credential_t credential;
    qiniu_ng_str_t access_key, secret_key;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(credential_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-3",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-3",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    putenv("QINIU_ACCESS_KEY=abcdefghklmnopq-4");
    putenv("QINIU_SECRET_KEY=1234567890-4");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(credential_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-4",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-4",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_free(&credential_provider);
}

void test_qiniu_ng_credential_provider_chain(void) {
    qiniu_ng_credential_provider_global_clear();
    putenv("QINIU_ACCESS_KEY=");
    putenv("QINIU_SECRET_KEY=");

    qiniu_ng_chain_credential_provider_builder_t builder = qiniu_ng_chain_credential_provider_builder_new();
    qiniu_ng_credential_provider_t global_provider = qiniu_ng_credential_provider_new_global();
    qiniu_ng_chain_credential_provider_builder_append_credential(&builder, &global_provider);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_credential_provider_is_null(global_provider), "global_provider != NULL");
    qiniu_ng_credential_provider_t env_provider = qiniu_ng_credential_provider_new_env();
    qiniu_ng_chain_credential_provider_builder_append_credential(&builder, &env_provider);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_credential_provider_is_null(env_provider), "env_provider != NULL");
    qiniu_ng_credential_provider_t static_provider = qiniu_ng_credential_provider_new_static("abcdefghklmnopq-s", "1234567890-s");
    qiniu_ng_chain_credential_provider_builder_append_credential(&builder, &static_provider);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_credential_provider_is_null(static_provider), "static_provider != NULL");
    qiniu_ng_credential_provider_t chain_provider = qiniu_ng_chain_credential_provider_build(&builder);

    qiniu_ng_credential_t credential;
    qiniu_ng_str_t access_key, secret_key;

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(chain_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-s",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-s",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    putenv("QINIU_ACCESS_KEY=abcdefghklmnopq-e");
    putenv("QINIU_SECRET_KEY=1234567890-e");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(chain_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-e",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-e",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_global_setup("abcdefghklmnopq-g", "1234567890-g");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(chain_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-g",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-g",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_global_clear();

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(chain_provider, &credential, NULL),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-e",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-e",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    qiniu_ng_credential_provider_free(&chain_provider);
}

static qiniu_ng_user_defined_credential_t mock_qiniu_ng_credential_provider(qiniu_ng_user_defined_credential_t init) {
    static unsigned long user_defined_credential_provider_id = 0;
    user_defined_credential_provider_id += 1;

    if (user_defined_credential_provider_id < 4) {
        char access_key[64], secret_key[64];
        snprintf(&access_key[0], 64, "abcdefghklmnopq-%ld", user_defined_credential_provider_id);
        snprintf(&secret_key[0], 64, "1234567890-%ld", user_defined_credential_provider_id);
        init.credential = qiniu_ng_credential_new(&access_key[0], &secret_key[0]);
    } else {
        init.error = user_defined_credential_provider_id;
    }

    return init;
}

void test_qiniu_ng_credential_provider_user_defined(void) {
    qiniu_ng_credential_provider_t user_defined_credential_provider = qiniu_ng_credential_provider_new_user_defined(mock_qiniu_ng_credential_provider);

    qiniu_ng_credential_t credential;
    qiniu_ng_str_t access_key, secret_key;
    qiniu_ng_err_t err;


    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(user_defined_credential_provider, &credential, &err),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-1",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-1",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(user_defined_credential_provider, &credential, &err),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-2",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-2",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_credential_provider_get(user_defined_credential_provider, &credential, &err),
        "qiniu_ng_credential_provider_get() RETURNS FALSE");
    access_key = qiniu_ng_credential_get_access_key(credential);
    secret_key = qiniu_ng_credential_get_secret_key(credential);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(access_key), "abcdefghklmnopq-3",
        "access_key RETURNS UNEXPECTED VALUE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_cstr(secret_key), "1234567890-3",
        "secret_key RETURNS UNEXPECTED VALUE");
    qiniu_ng_str_free(&access_key);
    qiniu_ng_str_free(&secret_key);
    qiniu_ng_credential_free(&credential);

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_credential_provider_get(user_defined_credential_provider, &credential, &err),
        "qiniu_ng_credential_provider_get() RETURNS TRUE");
    int err_code;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_io_error_extract(&err, &err_code),
        "err is not io_error");
    TEST_ASSERT_EQUAL_INT_MESSAGE(err_code, 4, "err_code != 4");

    qiniu_ng_credential_provider_free(&user_defined_credential_provider);
}
