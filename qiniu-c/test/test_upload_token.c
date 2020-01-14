#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_make_upload_token(void) {
    TEST_ASSERT_EQUAL_INT(env_load("..", false), 0);

    const qiniu_ng_char_t *CALLBACK_URLS[2] = {
        QINIU_NG_CHARS("https://apin1.qiniu.com/callback"),
        QINIU_NG_CHARS("https://apin2.qiniu.com/callback")
    };
    uint64_t deadline = (unsigned long long) time(NULL) + 86400;

    qiniu_ng_upload_policy_builder_t builder = qiniu_ng_new_upload_policy_builder_for_bucket(QINIU_NG_CHARS("test-bucket"), 86400);
    qiniu_ng_upload_policy_builder_set_insert_only(builder);
    qiniu_ng_upload_policy_builder_set_callback_urls(builder, (const qiniu_ng_char_t *const *) &CALLBACK_URLS[0], 2, NULL);
    qiniu_ng_upload_policy_builder_set_callback_body(builder, QINIU_NG_CHARS("key=$(key)"), NULL);
    qiniu_ng_upload_policy_t upload_policy = qiniu_ng_upload_policy_builder_build(&builder);
    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_builder_is_freed(builder));

    qiniu_ng_str_t bucket_name = qiniu_ng_upload_policy_get_bucket(upload_policy);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(bucket_name));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(bucket_name), QINIU_NG_CHARS("test-bucket"));
    qiniu_ng_str_free(&bucket_name);

    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_is_insert_only(upload_policy));
    TEST_ASSERT_FALSE(qiniu_ng_upload_policy_is_infrequent_storage_used(upload_policy));
    uint64_t deadline_1;
    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_get_token_deadline(upload_policy, &deadline_1));
    TEST_ASSERT_EQUAL_UINT(deadline, deadline_1);

    qiniu_ng_str_list_t callback_urls = qiniu_ng_upload_policy_get_callback_urls(upload_policy);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(callback_urls), 2);

    const qiniu_ng_char_t *callback_url;
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 0, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin1.qiniu.com/callback"));
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 1, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin2.qiniu.com/callback"));
    qiniu_ng_str_list_free(&callback_urls);

    qiniu_ng_str_t callback_body = qiniu_ng_upload_policy_get_callback_body(upload_policy);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(callback_body));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(callback_body), QINIU_NG_CHARS("key=$(key)"));
    qiniu_ng_str_free(&callback_body);

    qiniu_ng_str_t callback_body_type = qiniu_ng_upload_policy_get_callback_body_type(upload_policy);
    TEST_ASSERT_TRUE(qiniu_ng_str_is_null(callback_body_type));
    qiniu_ng_str_free(&callback_body_type);

    qiniu_ng_upload_token_t upload_token = qiniu_ng_new_upload_token_from_policy(upload_policy, GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));
    qiniu_ng_upload_policy_free(&upload_policy);

    qiniu_ng_str_t access_key;
    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_access_key(upload_token, &access_key, NULL));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(access_key), GETENV(QINIU_NG_CHARS("access_key")));
    qiniu_ng_str_free(&access_key);

    qiniu_ng_str_t token = qiniu_ng_upload_token_get_token(upload_token);
    TEST_ASSERT_EQUAL_INT(QINIU_NG_CHARS_NCMP(qiniu_ng_str_get_ptr(token), GETENV(QINIU_NG_CHARS("access_key")), QINIU_NG_CHARS_LEN(GETENV(QINIU_NG_CHARS("access_key")))), 0);
    qiniu_ng_str_free(&token);

    qiniu_ng_upload_policy_t upload_policy_2;
    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(upload_token, &upload_policy_2, NULL));

    bucket_name = qiniu_ng_upload_policy_get_bucket(upload_policy_2);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(bucket_name));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(bucket_name), QINIU_NG_CHARS("test-bucket"));
    qiniu_ng_str_free(&bucket_name);

    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_is_insert_only(upload_policy_2));
    TEST_ASSERT_FALSE(qiniu_ng_upload_policy_is_infrequent_storage_used(upload_policy_2));

    uint64_t deadline_2;
    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_get_token_deadline(upload_policy_2, &deadline_2));
    TEST_ASSERT_EQUAL_UINT(deadline, deadline_2);

    callback_urls = qiniu_ng_upload_policy_get_callback_urls(upload_policy_2);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(callback_urls), 2);

    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 0, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin1.qiniu.com/callback"));
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 1, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin2.qiniu.com/callback"));

    qiniu_ng_str_list_free(&callback_urls);

    callback_body = qiniu_ng_upload_policy_get_callback_body(upload_policy_2);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(callback_body));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(callback_body), QINIU_NG_CHARS("key=$(key)"));
    qiniu_ng_str_free(&callback_body);

    callback_body_type = qiniu_ng_upload_policy_get_callback_body_type(upload_policy_2);
    TEST_ASSERT_TRUE(qiniu_ng_str_is_null(callback_body_type));
    qiniu_ng_str_free(&callback_body_type);

    qiniu_ng_upload_policy_free(&upload_policy_2);

    token = qiniu_ng_upload_token_get_token(upload_token);
    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_token_t upload_token_2 = qiniu_ng_new_upload_token_from_token(qiniu_ng_str_get_ptr(token));
    qiniu_ng_str_free(&token);

    qiniu_ng_upload_policy_t upload_policy_3;
    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(upload_token_2, &upload_policy_3, NULL));
    qiniu_ng_upload_token_free(&upload_token_2);

    bucket_name = qiniu_ng_upload_policy_get_bucket(upload_policy_3);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(bucket_name));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(bucket_name), QINIU_NG_CHARS("test-bucket"));
    qiniu_ng_str_free(&bucket_name);

    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_is_insert_only(upload_policy_3));
    TEST_ASSERT_FALSE(qiniu_ng_upload_policy_is_infrequent_storage_used(upload_policy_3));
    uint64_t deadline_3;
    TEST_ASSERT_TRUE(qiniu_ng_upload_policy_get_token_deadline(upload_policy_3, &deadline_3));
    TEST_ASSERT_EQUAL_UINT(deadline, deadline_3);

    callback_urls = qiniu_ng_upload_policy_get_callback_urls(upload_policy_3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(callback_urls), 2);

    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 0, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin1.qiniu.com/callback"));
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(callback_urls, 1, &callback_url));
    TEST_ASSERT_EQUAL_STRING(callback_url, QINIU_NG_CHARS("https://apin2.qiniu.com/callback"));

    qiniu_ng_str_list_free(&callback_urls);

    callback_body = qiniu_ng_upload_policy_get_callback_body(upload_policy_3);
    TEST_ASSERT_FALSE(qiniu_ng_str_is_null(callback_body));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(callback_body), QINIU_NG_CHARS("key=$(key)"));
    qiniu_ng_str_free(&callback_body);

    callback_body_type = qiniu_ng_upload_policy_get_callback_body_type(upload_policy_3);
    TEST_ASSERT_TRUE(qiniu_ng_str_is_null(callback_body_type));
    qiniu_ng_str_free(&callback_body_type);

    qiniu_ng_upload_policy_free(&upload_policy_3);
}
