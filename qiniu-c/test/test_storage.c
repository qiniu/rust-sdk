#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_storage_bucket_names(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);

    qiniu_ng_string_list_t bucket_names;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_storage_bucket_names(client, &bucket_names, &err));

    unsigned int names_len = qiniu_ng_string_list_len(bucket_names);
    TEST_ASSERT_GREATER_THAN(5, names_len);
    for (unsigned int i = 0; i < names_len; i++) {
        const char *bucket_name;
        TEST_ASSERT_TRUE(qiniu_ng_string_list_get(bucket_names, i, &bucket_name));
    }
    qiniu_ng_string_list_free(bucket_names);
    qiniu_ng_client_free(client);
}

void test_qiniu_ng_storage_bucket_create_and_drop(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);

    const char buf[40];
    sprintf((char *) buf, "test-qiniu-c-%lu", (unsigned long) time(NULL));
    const char *new_bucket_name = (const char *) buf;

    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_storage_create_bucket(client, new_bucket_name, Z1, &err));

    qiniu_ng_string_list_t bucket_names;
    TEST_ASSERT_TRUE(qiniu_ng_storage_bucket_names(client, &bucket_names, &err));

    unsigned int names_len = qiniu_ng_string_list_len(bucket_names);
    TEST_ASSERT_GREATER_THAN(5, names_len);
    bool found_new_bucket = false;
    for (unsigned int i = 0; i < names_len; i++) {
        const char *bucket_name;
        TEST_ASSERT_TRUE(qiniu_ng_string_list_get(bucket_names, i, &bucket_name));
        if (strcmp(bucket_name, new_bucket_name) == 0) {
            found_new_bucket = true;
        }
    }
    qiniu_ng_string_list_free(bucket_names);
    TEST_ASSERT_TRUE(found_new_bucket);

    TEST_ASSERT_TRUE(qiniu_ng_storage_drop_bucket(client, new_bucket_name, &err));
    qiniu_ng_client_free(client);
}
