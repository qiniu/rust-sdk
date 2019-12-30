#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_make_upload_token(void) {
    env_load("..", false);

    const char *callback_urls[2] = {"https://apin1.qiniu.com/callback", "https://apin2.qiniu.com/callback"};
    qiniu_ng_err err;
    qiniu_ng_upload_policy_t policy = {
        .bucket = "test-bucket",
        .insert_only = true,
        .deadline = 1608877436,
        .callback_urls = (const char **) callback_urls,
        .callback_urls_len = 2,
#if defined(_WIN32) || defined(WIN32)
        .callback_body = L"key=$(key)"
#else
        .callback_body = "key=$(key)"
#endif
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));
    const char *t = qiniu_ng_upload_token_get_token(token);
    TEST_ASSERT_EQUAL_INT(strncmp(t, getenv("access_key"), strlen(getenv("access_key"))), 0);

    qiniu_ng_upload_policy_t policy2;
    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(token, &policy2, &err));
    TEST_ASSERT_EQUAL_STRING(policy2.bucket, policy.bucket);
    TEST_ASSERT_TRUE(policy2.insert_only);
    TEST_ASSERT_FALSE(policy2.infrequent_storage);
    TEST_ASSERT_EQUAL_INT(policy2.deadline, 1608877436);
    TEST_ASSERT_EQUAL_INT(policy2.callback_urls_len, 2);
    TEST_ASSERT_EQUAL_STRING_ARRAY(policy2.callback_urls, callback_urls, 2);
    TEST_ASSERT_NULL(policy2.callback_host);
    TEST_ASSERT_EQUAL_STRING(policy2.callback_body, policy.callback_body);
    TEST_ASSERT_NULL(policy2.callback_body_type);

    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(token, &policy2, &err));
    TEST_ASSERT_EQUAL_STRING(policy2.bucket, "test-bucket");
    TEST_ASSERT_TRUE(policy2.insert_only);
    TEST_ASSERT_FALSE(policy2.infrequent_storage);
    TEST_ASSERT_EQUAL_INT(policy2.deadline, 1608877436);
    TEST_ASSERT_EQUAL_INT(policy2.callback_urls_len, 2);
    TEST_ASSERT_EQUAL_STRING_ARRAY(policy2.callback_urls, callback_urls, 2);
    TEST_ASSERT_NULL(policy2.callback_host);
    TEST_ASSERT_EQUAL_STRING(policy2.callback_body, policy.callback_body);
    TEST_ASSERT_NULL(policy2.callback_body_type);

    qiniu_ng_upload_token_t token2 = qiniu_ng_new_upload_token_from_token(t);
    qiniu_ng_upload_token_free(token);

    qiniu_ng_upload_policy_t policy3;
    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(token2, &policy3, &err));
    TEST_ASSERT_EQUAL_STRING(policy3.bucket, "test-bucket");
    TEST_ASSERT_TRUE(policy3.insert_only);
    TEST_ASSERT_FALSE(policy3.infrequent_storage);
    TEST_ASSERT_EQUAL_INT(policy3.deadline, 1608877436);
    TEST_ASSERT_EQUAL_INT(policy3.callback_urls_len, 2);
    TEST_ASSERT_EQUAL_STRING_ARRAY(policy3.callback_urls, callback_urls, 2);
    TEST_ASSERT_NULL(policy3.callback_host);
    TEST_ASSERT_EQUAL_STRING(policy3.callback_body, policy.callback_body);
    TEST_ASSERT_NULL(policy3.callback_body_type);

    TEST_ASSERT_TRUE(qiniu_ng_upload_token_get_policy(token2, &policy3, &err));
    TEST_ASSERT_EQUAL_STRING(policy3.bucket, "test-bucket");
    TEST_ASSERT_TRUE(policy3.insert_only);
    TEST_ASSERT_FALSE(policy3.infrequent_storage);
    TEST_ASSERT_EQUAL_INT(policy3.deadline, 1608877436);
    TEST_ASSERT_EQUAL_INT(policy3.callback_urls_len, 2);
    TEST_ASSERT_EQUAL_STRING_ARRAY(policy3.callback_urls, callback_urls, 2);
    TEST_ASSERT_NULL(policy3.callback_host);
    TEST_ASSERT_EQUAL_STRING(policy3.callback_body, policy.callback_body);
    TEST_ASSERT_NULL(policy3.callback_body_type);
    qiniu_ng_upload_token_free(token2);
}
