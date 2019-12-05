#include "unity.h"
#include "libqiniu_ng.h"
#include <unistd.h>
#include <string.h>
#include "test.h"

void test_qiniu_ng_upload_file(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(upload_manager, "z0-bucket", getenv("access_key"), 5);

    const char buf[40];
    sprintf((char *) buf, "test-257m-%lu", (unsigned long) time(NULL));

    const char *file_path = create_temp_file(257 * 1024 * 1024);
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));
    TEST_ASSERT_TRUE(qiniu_ng_etag_from_file_path(file_path, (char *) &etag, NULL));

    qiniu_ng_upload_policy_t policy = {
        .bucket = "z0-bucket",
        .key = (const char *) buf,
        .insert_only = true,
        .deadline = (unsigned long) time(NULL) + 86400,
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));

    qiniu_ng_upload_params_t params = {
        .key = (const char *) buf,
    };
    qiniu_ng_upload_response_t upload_response;
    qiniu_ng_err err;
    TEST_ASSERT_TRUE(qiniu_ng_upload_file(bucket_uploader, token, file_path, (const char *) buf, NULL, &params, &upload_response, &err));
    TEST_ASSERT_NOT_NULL(qiniu_ng_upload_response_get_key(upload_response));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_upload_response_get_hash(upload_response), (const char *) &etag);

    qiniu_ng_upload_response_free(upload_response);
    qiniu_ng_upload_token_free(token);

    TEST_ASSERT_EQUAL_INT(unlink(file_path), 0);
    free((void *) file_path);

    qiniu_ng_bucket_uploader_free(bucket_uploader);
    qiniu_ng_upload_manager_free(upload_manager);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_upload_file_failed_by_mime(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(upload_manager, "z0-bucket", getenv("access_key"), 5);

    qiniu_ng_upload_policy_t policy = {
        .bucket = "z0-bucket",
        .deadline = (unsigned long) time(NULL) + 86400,
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));

    qiniu_ng_err err;
    TEST_ASSERT_FALSE(qiniu_ng_upload_file(bucket_uploader, token, "/dev/null", NULL, "invalid", NULL, NULL, &err));
    TEST_ASSERT_TRUE(qiniu_ng_err_is_bad_mime(&err));

    qiniu_ng_upload_token_free(token);

    qiniu_ng_bucket_uploader_free(bucket_uploader);
    qiniu_ng_upload_manager_free(upload_manager);
    qiniu_ng_config_free(config);
}
