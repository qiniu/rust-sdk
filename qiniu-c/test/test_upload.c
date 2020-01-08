#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include <stdio.h>
#include "test.h"

#if defined(_WIN32) || defined(WIN32)
#include <windows.h>
unsigned long last_print_time;
HANDLE mutex;
void print_progress(unsigned long long uploaded, unsigned long long total) {
    DWORD mutex_wait_result = WaitForSingleObject(mutex, INFINITE);
    switch (mutex_wait_result) {
    case WAIT_OBJECT_0:
        if (last_print_time + 5 < (unsigned long) time(NULL)) {
            printf("progress: %lld / %lld\n", uploaded, total);
            last_print_time = (unsigned long) time(NULL);
        }
	ReleaseMutex(mutex);
	break;
    case WAIT_ABANDONED:
	break;
    }
}
#else
#include <unistd.h>
#include <stdatomic.h>
atomic_ulong last_print_time;
void print_progress(unsigned long long uploaded, unsigned long long total) {
    if (last_print_time + 5 < (unsigned long) time(NULL)) {
        printf("progress: %lld / %lld\n", uploaded, total);
        last_print_time = (unsigned long) time(NULL);
    }
}
#endif

void test_qiniu_ng_upload_files(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(upload_manager, "z0-bucket", getenv("access_key"), 5);

    const qiniu_ng_char_t file_key[256];
#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_key, 256, L"测试-257m-%lu", (unsigned long) time(NULL));
#else
    sprintf((char *) file_key, "测试-257m-%lu", (unsigned long) time(NULL));
#endif

    const qiniu_ng_char_t *file_path = create_temp_file(257 * 1024 * 1024);
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));
    TEST_ASSERT_TRUE(qiniu_ng_etag_from_file_path(file_path, (char *) &etag, NULL));

    qiniu_ng_upload_policy_t policy = {
        .bucket = "z0-bucket",
        .insert_only = true,
        .deadline = (unsigned long) time(NULL) + 86400,
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));

    last_print_time = (unsigned long) time(NULL);
#if defined(_WIN32) || defined(WIN32)
    mutex = CreateMutex(NULL, FALSE, NULL);
#endif

    qiniu_ng_upload_params_t params = {
        .key = (const qiniu_ng_char_t *) file_key,
        .file_name = (const qiniu_ng_char_t *) file_key,
        .on_uploading_progress = print_progress,
    };
    qiniu_ng_upload_response_t upload_response;
    TEST_ASSERT_TRUE(qiniu_ng_upload_file_path(bucket_uploader, token, file_path, &params, &upload_response, NULL));

    qiniu_ng_optional_string_t key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(key));
    qiniu_ng_optional_string_free(key);

    qiniu_ng_optional_str_t hash = qiniu_ng_upload_response_get_hash(upload_response);
    TEST_ASSERT_FALSE(qiniu_ng_optional_str_is_null(hash));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_str_get_ptr(hash), (const char *) &etag);
    qiniu_ng_optional_str_free(hash);

    qiniu_ng_upload_response_free(upload_response);

    // TODO: Clean uploaded file
    last_print_time = (unsigned long) time(NULL);

#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_key, 256, L"测试-257m-%lu", (unsigned long) time(NULL));
    FILE *file = _wfopen(file_path, L"rb");
#else
    sprintf((char *) file_key, "测试-257m-%lu", (unsigned long) time(NULL));
    FILE *file = fopen(file_path, "r");
#endif
    TEST_ASSERT_NOT_NULL(file);
    TEST_ASSERT_TRUE(qiniu_ng_upload_file(bucket_uploader, token, file, &params, &upload_response, NULL));
    TEST_ASSERT_EQUAL_INT(fclose(file), 0);

    key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE(qiniu_ng_optional_string_is_null(key));
    qiniu_ng_optional_string_free(key);

    hash = qiniu_ng_upload_response_get_hash(upload_response);
    TEST_ASSERT_FALSE(qiniu_ng_optional_str_is_null(hash));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_optional_str_get_ptr(hash), (const char *) &etag);
    qiniu_ng_optional_str_free(hash);

    qiniu_ng_upload_response_free(upload_response);

    // TODO: Clean uploaded file

    qiniu_ng_upload_token_free(token);

#if defined(_WIN32) || defined(WIN32)
    TEST_ASSERT_EQUAL_INT(_wunlink(file_path), 0);
#else
    TEST_ASSERT_EQUAL_INT(unlink(file_path), 0);
#endif
    free((void *) file_path);

    qiniu_ng_bucket_uploader_free(bucket_uploader);
    qiniu_ng_upload_manager_free(upload_manager);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_upload_file_path_failed_by_mime(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(upload_manager, "z0-bucket", getenv("access_key"), 5);

    qiniu_ng_upload_policy_t policy = {
        .bucket = "z0-bucket",
        .deadline = (unsigned long) time(NULL) + 86400,
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));

    qiniu_ng_upload_params_t params = {
        .mime = "invalid"
    };
    qiniu_ng_err_t err;
    qiniu_ng_string_t error;

#if defined(_WIN32) || defined(WIN32)
    const wchar_t *file_path = L"/不存在的路径";
#else
    const char *file_path = "不存在的路径";
#endif
    TEST_ASSERT_FALSE(qiniu_ng_upload_file_path(bucket_uploader, token, file_path, &params, NULL, &err));
    TEST_ASSERT_TRUE(qiniu_ng_err_bad_mime_type_error_extract(&err, &error));
    qiniu_ng_string_free(error);
    TEST_ASSERT_FALSE(qiniu_ng_err_bad_mime_type_error_extract(&err, &error));

    qiniu_ng_upload_token_free(token);

    qiniu_ng_bucket_uploader_free(bucket_uploader);
    qiniu_ng_upload_manager_free(upload_manager);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_upload_file_path_failed_by_non_existed_path(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(upload_manager, "z0-bucket", getenv("access_key"), 5);

    qiniu_ng_upload_policy_t policy = {
        .bucket = "z0-bucket",
        .deadline = (unsigned long) time(NULL) + 86400,
    };
    qiniu_ng_upload_token_t token = qiniu_ng_new_upload_token_from_policy(&policy, getenv("access_key"), getenv("secret_key"));

    qiniu_ng_err_t err;
    int code;

#if defined(_WIN32) || defined(WIN32)
    const wchar_t *file_path = L"/不存在的路径";
#else
    const char *file_path = "不存在的路径";
#endif
    TEST_ASSERT_FALSE(qiniu_ng_upload_file_path(bucket_uploader, token, file_path, NULL, NULL, &err));
    TEST_ASSERT_FALSE(qiniu_ng_err_bad_mime_type_error_extract(&err, NULL));
    TEST_ASSERT_TRUE(qiniu_ng_err_os_error_extract(&err, &code));
    TEST_ASSERT_EQUAL_STRING(strerror(code), "No such file or directory");
    TEST_ASSERT_FALSE(qiniu_ng_err_os_error_extract(&err, &code));

    qiniu_ng_upload_token_free(token);

    qiniu_ng_bucket_uploader_free(bucket_uploader);
    qiniu_ng_upload_manager_free(upload_manager);
    qiniu_ng_config_free(config);
}
