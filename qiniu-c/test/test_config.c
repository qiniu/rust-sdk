#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"
#include <string.h>

void test_qiniu_ng_config_new_default(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_config_new(void) {
    qiniu_ng_config_t config;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(qiniu_ng_config_builder_new(), &config, NULL));

    TEST_ASSERT_TRUE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 1000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 22);

    qiniu_ng_optional_str_t user_agent = qiniu_ng_config_get_user_agent(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_str_is_null(user_agent));
    TEST_ASSERT_EQUAL_INT(strncmp(qiniu_ng_optional_str_get_ptr(user_agent), "QiniuRust/qiniu-ng-", strlen("QiniuRust/qiniu-ng-")), 0);
    qiniu_ng_optional_str_free(user_agent);

    qiniu_ng_str_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(rs_url), "https://rs.qbox.me");
    qiniu_ng_str_free(rs_url);

    qiniu_ng_str_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(uc_url), "https://uc.qbox.me");
    qiniu_ng_str_free(uc_url);

    qiniu_ng_str_t uplog_url = qiniu_ng_config_get_uplog_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(uplog_url), "https://uplog.qbox.me");
    qiniu_ng_str_free(uplog_url);

    TEST_ASSERT_TRUE(qiniu_ng_config_is_uplog_enabled(config));

    unsigned int upload_threshold;
    TEST_ASSERT_TRUE(qiniu_ng_config_get_uplog_file_upload_threshold(config, &upload_threshold));
    TEST_ASSERT_EQUAL_UINT(upload_threshold, 1 << 12);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 7);
    TEST_ASSERT_FALSE(qiniu_ng_config_get_upload_recorder_always_flush_records(config));

    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_resolutions_cache_lifetime(config), 60 * 60);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_auto_persistent_interval(config), 30 * 60);
    TEST_ASSERT_FALSE(qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config));

    qiniu_ng_config_free(config);
}

void test_qiniu_ng_config_new2(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();

    qiniu_ng_config_builder_user_agent(builder, "test-user-agent");
    qiniu_ng_config_builder_use_https(builder, false);
    qiniu_ng_config_builder_batch_max_operation_size(builder, 10000);
    qiniu_ng_config_builder_upload_threshold(builder, 1 << 23);
    qiniu_ng_config_builder_uc_host(builder, "uc.qiniu.com");
    qiniu_ng_config_builder_disable_uplog(builder);
    qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(builder, 60 * 60 * 24 * 5);
    qiniu_ng_config_builder_upload_recorder_always_flush_records(builder, true);
#if defined(_WIN32) || defined(WIN32)
    qiniu_ng_config_builder_upload_recorder_root_directory(builder, _wgetenv(L"USERPROFILE"));
#else
    qiniu_ng_config_builder_upload_recorder_root_directory(builder, getenv("HOME"));
#endif
    qiniu_ng_char_t* temp_file = create_temp_file(0);
    qiniu_ng_config_builder_create_new_domains_manager(builder, temp_file);
    free(temp_file);
    qiniu_ng_config_builder_domains_manager_url_frozen_duration(builder, 60 * 60 * 24);
    qiniu_ng_config_builder_domains_manager_disable_auto_persistent(builder);

    qiniu_ng_config_t config;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(builder, &config, NULL));

    TEST_ASSERT_FALSE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 10000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 23);

    qiniu_ng_optional_str_t user_agent = qiniu_ng_config_get_user_agent(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_str_is_null(user_agent));
    TEST_ASSERT_EQUAL_INT(strncmp(qiniu_ng_optional_str_get_ptr(user_agent), "QiniuRust/qiniu-ng-", strlen("QiniuRust/qiniu-ng-")), 0);
    TEST_ASSERT_NOT_NULL(strstr(qiniu_ng_optional_str_get_ptr(user_agent), "test-user-agent"));
    qiniu_ng_optional_str_free(user_agent);

    qiniu_ng_str_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(rs_url), "http://rs.qbox.me");
    qiniu_ng_str_free(rs_url);

    qiniu_ng_str_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(uc_url), "http://uc.qiniu.com");
    qiniu_ng_str_free(uc_url);

    qiniu_ng_str_t uplog_url = qiniu_ng_config_get_uplog_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(uplog_url), "https://uplog.qbox.me");
    qiniu_ng_str_free(uplog_url);

    TEST_ASSERT_FALSE(qiniu_ng_config_is_uplog_enabled(config));

    qiniu_ng_optional_string_t root_directory = qiniu_ng_config_get_upload_recorder_root_directory(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(root_directory));
#if defined(_WIN32) || defined(WIN32)
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_string_get_ptr(root_directory), _wgetenv(L"USERPROFILE"));
#else
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_string_get_ptr(root_directory), getenv("HOME"));
#endif
    qiniu_ng_optional_string_free(root_directory);

    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 5);
    TEST_ASSERT_TRUE(qiniu_ng_config_get_upload_recorder_always_flush_records(config));

    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_url_frozen_duration(config), 60 * 60 * 24);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_auto_persistent_interval(config), 0);
    TEST_ASSERT_TRUE(qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config));

    qiniu_ng_config_free(config);
}

static int before_action_counter, after_action_counter;

static bool test_qiniu_ng_config_http_request_before_action_handlers(qiniu_ng_http_request_t request) {
    before_action_counter++;
    qiniu_ng_http_request_set_custom_data(request, &before_action_counter);
    return true;
}

static bool test_qiniu_ng_config_http_request_after_action_handlers(qiniu_ng_http_request_t request, qiniu_ng_http_response_t response) {
    TEST_ASSERT_EQUAL_INT(before_action_counter, *((int *) qiniu_ng_http_request_get_custom_data(request)));
    (void)(response); // unuse response
    after_action_counter++;
    return true;
}

void test_qiniu_ng_config_http_request_handlers(void) {
    before_action_counter = 0;
    after_action_counter = 0;

    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();

    qiniu_ng_config_builder_append_http_request_before_action_handler(builder, test_qiniu_ng_config_http_request_before_action_handlers);
    qiniu_ng_config_builder_prepend_http_request_before_action_handler(builder, test_qiniu_ng_config_http_request_before_action_handlers);
    qiniu_ng_config_builder_append_http_request_after_action_handler(builder, test_qiniu_ng_config_http_request_after_action_handlers);

    qiniu_ng_config_t config;
    qiniu_ng_region_t region;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(builder, &config, NULL));

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, "z0-bucket");
    TEST_ASSERT_TRUE(qiniu_ng_bucket_get_region(bucket, &region, NULL));
    qiniu_ng_config_free(config);

    TEST_ASSERT_EQUAL_INT(before_action_counter, 2);
    TEST_ASSERT_EQUAL_INT(after_action_counter, 1);
}

