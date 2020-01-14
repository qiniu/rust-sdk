#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_storage_bucket_names(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    TEST_ASSERT_EQUAL_INT(env_load("..", false), 0);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);

    qiniu_ng_str_list_t bucket_names;
    TEST_ASSERT_TRUE(qiniu_ng_storage_bucket_names(client, &bucket_names, NULL));

    size_t names_len = qiniu_ng_str_list_len(bucket_names);
    TEST_ASSERT_GREATER_THAN(5, names_len);
    for (size_t i = 0; i < names_len; i++) {
        const qiniu_ng_char_t* bucket_name;
        TEST_ASSERT_TRUE(qiniu_ng_str_list_get(bucket_names, i, &bucket_name));
    }
    qiniu_ng_str_list_free(&bucket_names);
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_storage_bucket_create_and_drop(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    TEST_ASSERT_EQUAL_INT(env_load("..", false), 0);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);

    const qiniu_ng_char_t new_bucket_name[40];
#if defined(_WIN32) || defined(WIN32)
    swprintf((qiniu_ng_char_t *) new_bucket_name, 40, L"test-qiniu-c-%lld", (long long) time(NULL));
#else
    snprintf((qiniu_ng_char_t *) new_bucket_name, 40, "test-qiniu-c-%lld", (long long) time(NULL));
#endif

    TEST_ASSERT_TRUE(qiniu_ng_storage_create_bucket(client, new_bucket_name, qiniu_ng_region_z1, NULL));

    qiniu_ng_str_list_t bucket_names;
    TEST_ASSERT_TRUE(qiniu_ng_storage_bucket_names(client, &bucket_names, NULL));

    size_t names_len = qiniu_ng_str_list_len(bucket_names);
    TEST_ASSERT_GREATER_THAN(5, names_len);
    bool found_new_bucket = false;
    for (size_t i = 0; i < names_len; i++) {
        const qiniu_ng_char_t *bucket_name;
        TEST_ASSERT_TRUE(qiniu_ng_str_list_get(bucket_names, i, &bucket_name));
        if (QINIU_NG_CHARS_CMP(bucket_name, new_bucket_name) == 0) {
            found_new_bucket = true;
        }
    }
    qiniu_ng_str_list_free(&bucket_names);
    TEST_ASSERT_TRUE(found_new_bucket);

    TEST_ASSERT_TRUE(qiniu_ng_storage_drop_bucket(client, new_bucket_name, NULL));
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_storage_bucket_create_duplicated(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    TEST_ASSERT_EQUAL_INT(env_load("..", false), 0);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);

    qiniu_ng_err_t err;
    unsigned short code;
    qiniu_ng_str_t error_message;

    TEST_ASSERT_FALSE(qiniu_ng_storage_create_bucket(client, QINIU_NG_CHARS("z0-bucket"), qiniu_ng_region_z1, &err));
    TEST_ASSERT_FALSE(qiniu_ng_err_os_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_io_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_json_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_unknown_error_extract(&err, NULL));
    TEST_ASSERT_TRUE(qiniu_ng_err_response_status_code_error_extract(&err, &code, &error_message));
    TEST_ASSERT_EQUAL_UINT(code, 614);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(error_message), QINIU_NG_CHARS("the bucket already exists and you own it."));
    TEST_ASSERT_FALSE(qiniu_ng_err_response_status_code_error_extract(&err, NULL, NULL));

    qiniu_ng_str_free(&error_message);
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);
}
