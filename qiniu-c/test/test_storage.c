#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_storage_bucket_names(void) {
    qiniu_ng_config config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);

    qiniu_ng_string_list_t bucket_names;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_storage_bucket_names(client, &bucket_names, &err));

    unsigned int names_len = qiniu_ng_string_list_len(bucket_names);
    TEST_ASSERT_TRUE(names_len > 5);
    for (unsigned int i = 0; i < names_len; i++) {
        const char *bucket_name;
        TEST_ASSERT_TRUE(qiniu_ng_string_list_get(bucket_names, i, &bucket_name));
    }
    qiniu_ng_string_list_free(bucket_names);
}
