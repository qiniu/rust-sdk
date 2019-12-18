#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config_new_default(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_config_new(void) {
    qiniu_ng_config_t config;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(qiniu_ng_config_builder_new(), &config, &err));

    TEST_ASSERT_FALSE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 1000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 22);

    qiniu_ng_string_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "http://rs.qiniu.com");
    qiniu_ng_string_free(rs_url);

    qiniu_ng_string_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(uc_url), "http://uc.qbox.me");
    qiniu_ng_string_free(uc_url);

    TEST_ASSERT_TRUE(qiniu_ng_config_is_uplog_enabled(config));
    qiniu_ng_optional_string_t uplog_server_url = qiniu_ng_config_get_uplog_server_url(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(uplog_server_url));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_string_get_ptr(uplog_server_url), "https://uplog.qbox.me");
    qiniu_ng_optional_string_free(uplog_server_url);

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

    qiniu_ng_config_builder_use_https(builder, true);
    qiniu_ng_config_builder_batch_max_operation_size(builder, 10000);
    qiniu_ng_config_builder_upload_threshold(builder, 1 << 23);
    qiniu_ng_config_builder_uc_host(builder, "uc.qiniu.com");
    qiniu_ng_config_builder_disable_uplog(builder);
    qiniu_ng_config_builder_upload_recorder_root_directory(builder,getenv("HOME"));
    qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(builder, 60 * 60 * 24 * 5);
    qiniu_ng_config_builder_upload_recorder_always_flush_records(builder, true);
    qiniu_ng_config_builder_create_new_domains_manager(builder, "/tmp/persistent_file");
    qiniu_ng_config_builder_domains_manager_url_frozen_duration(builder, 60 * 60 * 24);
    qiniu_ng_config_builder_domains_manager_disable_auto_persistent(builder);

    qiniu_ng_config_t config;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(builder, &config, &err));

    TEST_ASSERT_TRUE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 10000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 23);

    qiniu_ng_string_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "https://rs.qiniu.com");
    qiniu_ng_string_free(rs_url);

    qiniu_ng_string_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(uc_url), "https://uc.qiniu.com");
    qiniu_ng_string_free(uc_url);

    TEST_ASSERT_FALSE(qiniu_ng_config_is_uplog_enabled(config));
    qiniu_ng_optional_string_t uplog_server_url = qiniu_ng_config_get_uplog_server_url(config);
    TEST_ASSERT_TRUE(qiniu_ng_optional_string_is_null(uplog_server_url));
    qiniu_ng_optional_string_free(uplog_server_url);

    qiniu_ng_optional_string_t root_directory = qiniu_ng_config_get_upload_recorder_root_directory(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(root_directory));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_string_get_ptr(root_directory), getenv("HOME"));
    qiniu_ng_optional_string_free(root_directory);

    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 5);
    TEST_ASSERT_TRUE(qiniu_ng_config_get_upload_recorder_always_flush_records(config));

    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_resolutions_cache_lifetime(config), 60 * 60);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_domains_manager_auto_persistent_interval(config), 0);
    TEST_ASSERT_TRUE(qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config));

    qiniu_ng_config_free(config);
}
