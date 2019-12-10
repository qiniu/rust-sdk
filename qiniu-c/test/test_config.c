#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config_new_default(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_config_new(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();

    TEST_ASSERT_FALSE(qiniu_ng_config_builder_get_use_https(builder));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_builder_get_batch_max_operation_size(builder), 1000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_builder_get_upload_threshold(builder), 1 << 22);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_config_builder_get_rs_host(builder), "rs.qiniu.com");
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_config_builder_get_uc_host(builder), "uc.qbox.me");
    TEST_ASSERT_TRUE(qiniu_ng_config_builder_is_uplog_enabled(builder));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_config_builder_get_uplog_server_url(builder), "https://uplog.qbox.me");

    uint upload_threshold;
    TEST_ASSERT_TRUE(qiniu_ng_config_builder_get_uplog_file_upload_threshold(builder, &upload_threshold));
    TEST_ASSERT_EQUAL_UINT(upload_threshold, 1 << 12);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_builder_get_upload_recorder_upload_block_lifetime(builder), 60 * 60 * 24 * 7);

    qiniu_ng_config_t config;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(builder, &config, &err));

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

    TEST_ASSERT_TRUE(qiniu_ng_config_get_uplog_file_upload_threshold(config, &upload_threshold));
    TEST_ASSERT_EQUAL_UINT(upload_threshold, 1 << 12);
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 7);

    qiniu_ng_config_free(config);
}

void test_qiniu_ng_config_new2(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_set_use_https(builder, true);
    qiniu_ng_config_builder_set_batch_max_operation_size(builder, 10000);
    qiniu_ng_config_builder_set_rs_host(builder, "rspub.qiniu.com");
    qiniu_ng_config_builder_disable_uplog(builder);
    qiniu_ng_config_builder_set_upload_recorder_upload_block_lifetime(builder, 60 * 60 * 24 * 7 * 365);

    qiniu_ng_config_t config;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_config_build(builder, &config, &err));

    TEST_ASSERT_TRUE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 10000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 22);

    qiniu_ng_string_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "https://rspub.qiniu.com");
    qiniu_ng_string_free(rs_url);

    qiniu_ng_string_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(uc_url), "https://uc.qbox.me");
    qiniu_ng_string_free(uc_url);

    TEST_ASSERT_FALSE(qiniu_ng_config_is_uplog_enabled(config));
    qiniu_ng_optional_string_t uplog_server_url = qiniu_ng_config_get_uplog_server_url(config);
    TEST_ASSERT_TRUE(qiniu_ng_optional_string_is_null(uplog_server_url));
    qiniu_ng_optional_string_free(uplog_server_url);

    qiniu_ng_optional_string_t root_directory = qiniu_ng_config_get_upload_recorder_root_directory(config);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(root_directory));
    qiniu_ng_optional_string_free(root_directory);

    uint upload_threshold;
    TEST_ASSERT_FALSE(qiniu_ng_config_get_uplog_file_upload_threshold(config, &upload_threshold));
    TEST_ASSERT_EQUAL_UINT(qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 7 * 365);

    qiniu_ng_config_free(config);
}
