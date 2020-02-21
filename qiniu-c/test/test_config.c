#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"
#include <string.h>
#include <errno.h>

void test_qiniu_ng_config_new_default(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_config_new(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_build(&builder, &config, NULL),
        "qiniu_ng_config_build() failed");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_is_freed(builder),
        "qiniu_ng_config_builder_is_freed() failed");

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_get_use_https(config),
        "qiniu_ng_config_get_use_https() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_config_get_batch_max_operation_size(config), 1000,
        "qiniu_ng_config_get_batch_max_operation_size() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_config_get_upload_threshold(config), 1 << 22,
        "qiniu_ng_config_get_upload_threshold() returns unexpected value");

    qiniu_ng_str_t user_agent = qiniu_ng_config_get_user_agent(config);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        QINIU_NG_CHARS_NCMP(qiniu_ng_str_get_ptr(user_agent), QINIU_NG_CHARS("QiniuRust/qiniu-ng-"), QINIU_NG_CHARS_LEN(QINIU_NG_CHARS("QiniuRust/qiniu-ng-"))),
        0,
        "qiniu_ng_str_get_ptr(user_agent) has not prefix \"QiniuRust/qiniu-ng-\"");
    qiniu_ng_str_free(&user_agent);

    qiniu_ng_str_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(rs_url), QINIU_NG_CHARS("https://rs.qbox.me"),
        "qiniu_ng_str_get_ptr(rs_url) != \"https://rs.qbox.me\"");
    qiniu_ng_str_free(&rs_url);

    qiniu_ng_str_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(uc_url), QINIU_NG_CHARS("https://uc.qbox.me"),
        "qiniu_ng_str_get_ptr(uc_url) != \"https://uc.qbox.me\"");
    qiniu_ng_str_free(&uc_url);

    qiniu_ng_str_t uplog_url = qiniu_ng_config_get_uplog_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(uplog_url), QINIU_NG_CHARS("https://uplog.qbox.me"),
        "qiniu_ng_str_get_ptr(uplog_url) != \"https://uplog.qbox.me\"");
    qiniu_ng_str_free(&uplog_url);

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_is_uplog_enabled(config),
        "qiniu_ng_config_is_uplog_enabled() returns unexpected value");

    uint32_t upload_threshold;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_get_uplog_file_upload_threshold(config, &upload_threshold),
        "qiniu_ng_config_get_uplog_file_upload_threshold() failed");
    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        upload_threshold, 1 << 12,
        "upload_threshold != 1<<12");
    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 7,
        "qiniu_ng_config_get_upload_recorder_upload_block_lifetime() returns unexpected value");
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_config_get_upload_recorder_always_flush_records(config),
        "qiniu_ng_config_get_upload_recorder_always_flush_records() returns unexpected value");

    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_domains_manager_resolutions_cache_lifetime(config), 60 * 60,
        "qiniu_ng_config_get_domains_manager_resolutions_cache_lifetime() returns unexpected value");
    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_domains_manager_auto_persistent_interval(config), 30 * 60,
        "qiniu_ng_config_get_domains_manager_auto_persistent_interval() returns unexpected value");
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config),
        "qiniu_ng_config_get_domains_manager_auto_persistent_disabled() returns unexpected value");

    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_config_new2(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();

    qiniu_ng_config_builder_set_appended_user_agent(builder, QINIU_NG_CHARS("test-user-agent"));
    qiniu_ng_config_builder_use_https(builder, false);
    qiniu_ng_config_builder_batch_max_operation_size(builder, 10000);
    qiniu_ng_config_builder_upload_threshold(builder, 1 << 23);
    qiniu_ng_config_builder_uc_host(builder, QINIU_NG_CHARS("uc.qiniu.com"));
    qiniu_ng_config_builder_disable_uplog(builder);
    qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(builder, 60 * 60 * 24 * 5);
    qiniu_ng_config_builder_upload_recorder_always_flush_records(builder, true);
#if defined(_WIN32) || defined(WIN32)
    const qiniu_ng_char_t *home_directory = GETENV(QINIU_NG_CHARS("USERPROFILE"));
#else
    const qiniu_ng_char_t *home_directory = GETENV(QINIU_NG_CHARS("HOME"));
#endif
    qiniu_ng_config_builder_upload_recorder_root_directory(builder, home_directory);
    qiniu_ng_char_t* temp_file = create_temp_file(0);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_create_new_domains_manager(builder, temp_file, NULL),
        "qiniu_ng_config_builder_create_new_domains_manager() failed");
    free(temp_file);
    qiniu_ng_config_builder_domains_manager_url_frozen_duration(builder, 60 * 60 * 24);
    qiniu_ng_config_builder_domains_manager_disable_auto_persistent(builder);

    qiniu_ng_config_t config;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_build(&builder, &config, NULL),
        "qiniu_ng_config_build() failed");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_is_freed(builder),
        "qiniu_ng_config_builder_is_freed() failed");

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_config_get_use_https(config),
        "qiniu_ng_config_get_use_https() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_config_get_batch_max_operation_size(config), 10000,
        "qiniu_ng_config_get_batch_max_operation_size() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_config_get_upload_threshold(config), 1 << 23,
        "qiniu_ng_config_get_upload_threshold() returns unexpected value");

    qiniu_ng_str_t user_agent = qiniu_ng_config_get_user_agent(config);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        QINIU_NG_CHARS_NCMP(qiniu_ng_str_get_ptr(user_agent), QINIU_NG_CHARS("QiniuRust/qiniu-ng-"), QINIU_NG_CHARS_LEN(QINIU_NG_CHARS("QiniuRust/qiniu-ng-"))),
        0,
        "qiniu_ng_str_get_ptr(user_agent) has not prefix \"QiniuRust/qiniu-ng-\"");
    TEST_ASSERT_NOT_NULL_MESSAGE(
        QINIU_NG_CHARS_STR(qiniu_ng_str_get_ptr(user_agent), QINIU_NG_CHARS("test-user-agent")),
        "qiniu_ng_str_get_ptr(user_agent) does not contain \"test-user-agent\"");
    qiniu_ng_str_free(&user_agent);

    qiniu_ng_str_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(rs_url), QINIU_NG_CHARS("http://rs.qbox.me"),
        "qiniu_ng_str_get_ptr(rs_url) != \"http://rs.qbox.me\"");
    qiniu_ng_str_free(&rs_url);

    qiniu_ng_str_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(uc_url), QINIU_NG_CHARS("http://uc.qiniu.com"),
        "qiniu_ng_str_get_ptr(uc_url) != \"http://uc.qiniu.com\"");
    qiniu_ng_str_free(&uc_url);

    qiniu_ng_str_t uplog_url = qiniu_ng_config_get_uplog_url(config);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(uplog_url), QINIU_NG_CHARS("http://uplog.qbox.me"),
        "qiniu_ng_str_get_ptr(uplog_url) != \"http://uplog.qbox.me\"");
    qiniu_ng_str_free(&uplog_url);

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_config_is_uplog_enabled(config),
        "qiniu_ng_config_is_uplog_enabled() returns unexpected value");

    qiniu_ng_str_t root_directory = qiniu_ng_config_get_upload_recorder_root_directory(config);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_str_is_null(root_directory),
        "qiniu_ng_str_is_null(root_directory) returns unexpected value");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(root_directory), home_directory,
        "qiniu_ng_str_get_ptr(root_directory) != home_directory");
    qiniu_ng_str_free(&root_directory);

    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config), 60 * 60 * 24 * 5,
        "qiniu_ng_config_get_upload_recorder_upload_block_lifetime() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_get_upload_recorder_always_flush_records(config),
        "qiniu_ng_config_get_upload_recorder_always_flush_records() returns unexpected value");

    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_domains_manager_url_frozen_duration(config), 60 * 60 * 24,
        "qiniu_ng_config_get_domains_manager_url_frozen_duration() returns unexpected value");
    TEST_ASSERT_EQUAL_UINT_MESSAGE(
        qiniu_ng_config_get_domains_manager_auto_persistent_interval(config), 0,
        "qiniu_ng_config_get_domains_manager_auto_persistent_interval() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config),
        "qiniu_ng_config_get_domains_manager_auto_persistent_disabled() returns unexpected value");

    qiniu_ng_config_free(&config);
}

static int before_action_counter, after_action_counter;

static void test_qiniu_ng_config_http_request_before_action_handlers(qiniu_ng_http_request_t request, qiniu_ng_callback_err_t *err) {
    before_action_counter++;
    qiniu_ng_http_request_set_custom_data(request, &before_action_counter);

    qiniu_ng_str_map_t headers = qiniu_ng_http_request_get_headers(request);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(headers, QINIU_NG_CHARS("Accept")), QINIU_NG_CHARS("application/json"),
        "headers[\"Accept\"] != \"application/json\"");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(headers, QINIU_NG_CHARS("Content-Type")), QINIU_NG_CHARS("application/x-www-form-urlencoded"),
        "headers[\"Content-Type\"] != \"application/x-www-form-urlencoded\"");
    qiniu_ng_str_map_free(&headers);
    (void)(err);
}

static void test_qiniu_ng_config_http_request_after_action_handlers(qiniu_ng_http_request_t request, qiniu_ng_http_response_t response, qiniu_ng_callback_err_t *err) {
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        before_action_counter, *((int *) qiniu_ng_http_request_get_custom_data(request)),
        "qiniu_ng_http_request_get_custom_data() returns unexpected value");
    after_action_counter++;

    uint64_t body_len;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_http_response_get_body_length(response, &body_len, NULL),
        "qiniu_ng_http_response_get_body_length() failed");
    TEST_ASSERT_GREATER_THAN_UINT_MESSAGE(
        1, body_len,
        "body_len != 1");
    char* body = (char *) malloc((size_t) body_len);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_http_response_dump_body(response, body_len, body, &body_len, NULL),
        "qiniu_ng_http_response_dump_body() failed");
    TEST_ASSERT_GREATER_THAN_UINT_MESSAGE(
        1, body_len,
        "body_len != 1");

    qiniu_ng_char_t* temp_file_path = create_temp_file(0);

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_http_response_dump_body_to_file(response, temp_file_path, NULL),
        "qiniu_ng_http_response_dump_body_to_file() failed");

    char etag[ETAG_SIZE], etag2[ETAG_SIZE];
    qiniu_ng_etag_from_buffer(body, body_len, (char *) &etag);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_etag_from_file_path(temp_file_path, (char *) &etag2, NULL),
        "qiniu_ng_etag_from_file_path() failed");
    TEST_ASSERT_EQUAL_INT_MESSAGE(strncmp(etag, etag2, ETAG_SIZE), 0, "etag != etag2");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_http_response_set_body_to_file(response, temp_file_path, NULL),
        "qiniu_ng_http_response_set_body_to_file() failed");
    free((void *) temp_file_path);
    (void)(err);
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
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_build(&builder, &config, NULL),
        "qiniu_ng_config_build() failed");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_is_freed(builder),
        "qiniu_ng_config_builder_is_freed() failed");

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_bucket_get_region(bucket, &region, NULL),
        "qiniu_ng_bucket_get_region() failed");
    qiniu_ng_region_free(&region);
    qiniu_ng_bucket_free(&bucket);
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);

    TEST_ASSERT_EQUAL_INT_MESSAGE(
        before_action_counter, 2,
        "before_action_counter != 2");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        after_action_counter, 1,
        "after_action_counter != 1");
}

static int32_t qiniu_ng_readable_always_returns_error(void *context, void *buf, size_t count, size_t *have_read) {
    (void)(context);
    (void)(buf);
    (void)(count);
    (void)(have_read);
    return EACCES;
}

static void test_qiniu_ng_config_bad_http_request_after_action_handlers(qiniu_ng_http_request_t request, qiniu_ng_http_response_t response, qiniu_ng_callback_err_t *err) {
    (void)(request);
    qiniu_ng_readable_t reader = {
        .read_func = qiniu_ng_readable_always_returns_error,
        .context = NULL
    };
    qiniu_ng_http_response_set_body_to_reader(response, reader);
    (void)(err);
}

void test_qiniu_ng_config_bad_http_request_handlers(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_append_http_request_after_action_handler(builder, test_qiniu_ng_config_bad_http_request_after_action_handlers);

    qiniu_ng_config_t config;
    qiniu_ng_err_t err;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_build(&builder, &config, NULL),
        "qiniu_ng_config_build() failed");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_is_freed(builder),
        "qiniu_ng_config_builder_is_freed() failed");

    int32_t code;
    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_bucket_get_region(bucket, NULL, &err),
        "qiniu_ng_bucket_get_region() returns unexpected value");
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_err_curl_error_extract(&err, NULL),
        "qiniu_ng_err_curl_error_extract() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &code),
        "qiniu_ng_err_os_error_extract() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(code, EACCES, "code != EACCES");
    qiniu_ng_bucket_free(&bucket);
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);
}

static void test_qiniu_ng_config_http_request_after_action_handlers_always_return_error(qiniu_ng_http_request_t request, qiniu_ng_http_response_t response, qiniu_ng_callback_err_t *err) {
    (void)(request);
    (void)(response);
    err->error = qiniu_ng_err_os_error_new(EPERM);
    err->retry_kind = qiniu_ng_retry_kind_unretryable_error;
}

void test_qiniu_ng_config_bad_http_request_handlers_2(void) {
    qiniu_ng_config_builder_t builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_append_http_request_after_action_handler(builder, test_qiniu_ng_config_http_request_after_action_handlers_always_return_error);

    qiniu_ng_config_t config;
    qiniu_ng_err_t err;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_build(&builder, &config, NULL),
        "qiniu_ng_config_build() failed");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_config_builder_is_freed(builder),
        "qiniu_ng_config_builder_is_freed() failed");

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_bucket_get_region(bucket, NULL, &err),
        "qiniu_ng_bucket_get_region() returns unexpected value");
    int32_t code;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &code),
        "qiniu_ng_err_user_canceled_error_extract() returns unexpected value");
    TEST_ASSERT_EQUAL_INT_MESSAGE(code, EPERM, "code != EPERM");
    qiniu_ng_bucket_free(&bucket);
    qiniu_ng_client_free(&client);
    qiniu_ng_config_free(&config);
}
