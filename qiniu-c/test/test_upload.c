#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include <stdio.h>
#include "test.h"

#if defined(_WIN32) || defined(WIN32)
#include <windows.h>
long long last_print_time;
HANDLE mutex;
void print_progress(uint64_t uploaded, uint64_t total) {
    DWORD mutex_wait_result = WaitForSingleObject(mutex, INFINITE);
    switch (mutex_wait_result) {
    case WAIT_OBJECT_0:
        if (last_print_time + 5 < (long long) time(NULL)) {
            printf("%d: progress: %llu / %llu\n", GetCurrentThreadId(), uploaded, total);
            last_print_time = (long long) time(NULL);
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
#include <pthread.h>
atomic_llong last_print_time;
void print_progress(uint64_t uploaded, uint64_t total) {
    if (last_print_time + 5 < (long long) time(NULL)) {
        printf("%d: progress: %llu / %llu\n", (int) pthread_self(), uploaded, total);
        last_print_time = (long long) time(NULL);
    }
}
#endif

void test_qiniu_ng_upload_files(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(
        upload_manager, QINIU_NG_CHARS("z0-bucket"), GETENV(QINIU_NG_CHARS("access_key")), 5);

    const qiniu_ng_char_t file_key[256];
#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_key, 256, L"测试-513m-%lld", (long long) time(NULL));
#else
    snprintf((char *) file_key, 256, "测试-513m-%lld", (long long) time(NULL));
#endif

    const qiniu_ng_char_t *file_path = create_temp_file(513 * 1024 * 1024);
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_etag_from_file_path(file_path, (char *) &etag[0], NULL),
        "qiniu_ng_etag_from_file_path() failed");

    qiniu_ng_upload_policy_builder_t policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(QINIU_NG_CHARS("z0-bucket"), config);
    qiniu_ng_upload_policy_builder_set_insert_only(policy_builder);
    qiniu_ng_upload_token_t token = qiniu_ng_upload_token_new_from_policy_builder(policy_builder, GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));

    last_print_time = (long long) time(NULL);
#if defined(_WIN32) || defined(WIN32)
    mutex = CreateMutex(NULL, FALSE, NULL);
#endif

    last_print_time = (long long) time(NULL);
    qiniu_ng_upload_params_t params = {
        .key = (const qiniu_ng_char_t *) &file_key[0],
        .file_name = (const qiniu_ng_char_t *) &file_key[0],
        .on_uploading_progress = print_progress,
    };
    qiniu_ng_upload_response_t upload_response;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_bucket_uploader_upload_file_path(bucket_uploader, token, file_path, &params, &upload_response, NULL),
        "qiniu_ng_bucket_uploader_upload_file_path() failed");

    qiniu_ng_str_t key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_str_is_null(key),
        "qiniu_ng_str_is_null(key) != false");
    qiniu_ng_str_free(&key);

    char hash[ETAG_SIZE + 1];
    size_t hash_size;
    memset(hash, 0, ETAG_SIZE + 1);
    qiniu_ng_upload_response_get_hash(upload_response, (char *) &hash[0], &hash_size);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        hash_size, ETAG_SIZE,
        "hash_size != ETAG_SIZE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        hash, (const char *) &etag,
        "hash != etag");

    qiniu_ng_upload_response_free(&upload_response);

    // TODO: Clean uploaded file
    last_print_time = (long long) time(NULL);

#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_key, 256, L"测试-513m-%lld", (long long) time(NULL));
#else
    snprintf((char *) file_key, 256, "测试-513m-%lld", (long long) time(NULL));
#endif
    FILE *file = OPEN_FILE_FOR_READING(file_path);
    TEST_ASSERT_NOT_NULL_MESSAGE(
        file,
        "file == null");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_bucket_uploader_upload_file(bucket_uploader, token, file, &params, &upload_response, NULL),
        "qiniu_ng_bucket_uploader_upload_file() failed");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        fclose(file), 0,
        "fclose(file) != 0");

    key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_str_is_null(key),
        "qiniu_ng_str_is_null(key) != false");
    qiniu_ng_str_free(&key);

    memset(hash, 0, ETAG_SIZE + 1);
    qiniu_ng_upload_response_get_hash(upload_response, (char *) &hash[0], &hash_size);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        hash_size, ETAG_SIZE,
        "hash_size != ETAG_SIZE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        hash, (const char *) &etag,
        "hash != etag");

    qiniu_ng_upload_response_free(&upload_response);

    // TODO: Clean uploaded file

    qiniu_ng_upload_token_free(&token);

    DELETE_FILE(file_path);
    free((void *) file_path);

    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
}

struct upload_file_thread_context {
    const qiniu_ng_char_t *key;
    const qiniu_ng_char_t *file_path;
    const char *etag;
    qiniu_ng_bucket_uploader_t bucket_uploader;
    qiniu_ng_upload_token_t token;
};

#if defined(_WIN32) || defined(WIN32)
void *thread_of_upload_file(void* data);
DWORD WINAPI ThreadOfUploadFile(LPVOID data) {
    thread_of_upload_file((void*) data);
    return 0;
}
#endif

void *thread_of_upload_file(void* data) {
    struct upload_file_thread_context *context = (struct upload_file_thread_context *) data;
    qiniu_ng_upload_params_t params = {
        .key = (const qiniu_ng_char_t *) context->key,
        .file_name = (const qiniu_ng_char_t *) context->key,
        .on_uploading_progress = print_progress,
    };
    qiniu_ng_upload_response_t upload_response;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_bucket_uploader_upload_file_path(context->bucket_uploader, context->token, context->file_path, &params, &upload_response, NULL),
        "qiniu_ng_bucket_uploader_upload_file_path() failed");

    qiniu_ng_str_t key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_str_is_null(key),
        "qiniu_ng_str_is_null(key) != false");
    qiniu_ng_str_free(&key);

    char hash[ETAG_SIZE + 1];
    size_t hash_size;
    memset(hash, 0, ETAG_SIZE + 1);
    qiniu_ng_upload_response_get_hash(upload_response, (char *) &hash[0], &hash_size);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        hash_size, ETAG_SIZE,
        "hash_size != ETAG_SIZE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        hash, (const char *) context->etag,
        "hash != etag");

    qiniu_ng_upload_response_free(&upload_response);
    return NULL;
}

void test_qiniu_ng_upload_huge_number_of_files(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(
        upload_manager, QINIU_NG_CHARS("z0-bucket"), GETENV(QINIU_NG_CHARS("access_key")), 5);

    const qiniu_ng_char_t *file_path = create_temp_file(4 * 1024 * 1024 + 1);

    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_etag_from_file_path(file_path, (char *) &etag[0], NULL),
        "qiniu_ng_etag_from_file_path() failed");

    qiniu_ng_upload_policy_builder_t policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(QINIU_NG_CHARS("z0-bucket"), config);
    qiniu_ng_upload_policy_builder_set_insert_only(policy_builder);
    qiniu_ng_upload_token_t token = qiniu_ng_upload_token_new_from_policy_builder(policy_builder, GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));

    last_print_time = (long long) time(NULL);
#if defined(_WIN32) || defined(WIN32)
    mutex = CreateMutex(NULL, FALSE, NULL);
#endif

#define THREAD_COUNT (128)
    struct upload_file_thread_context contexts[THREAD_COUNT];
    char *keys[THREAD_COUNT];
    for (int i = 0; i < THREAD_COUNT; i++) {
        keys[i] = malloc(256 * sizeof(qiniu_ng_char_t));
#if defined(_WIN32) || defined(WIN32)
        swprintf((wchar_t *) keys[i], 256, L"测试-4m-%d-%lld", i, (long long) time(NULL));
#else
        snprintf((char *) keys[i], 256, "测试-4m-%d-%lld", i, (long long) time(NULL));
#endif
        contexts[i] = (struct upload_file_thread_context) {
            .key = keys[i],
            .file_path = file_path,
            .etag = (char *) &etag[0],
            .bucket_uploader = bucket_uploader,
            .token = token,
        };
    }
#if defined(_WIN32) || defined(WIN32)
    DWORD thread_ids[THREAD_COUNT];
    HANDLE threads[THREAD_COUNT];
    for (int i = 0; i < THREAD_COUNT; i++) {
	threads[i] = CreateThread(NULL, 0, ThreadOfUploadFile, &contexts[i], 0, &thread_ids[i]);
	TEST_ASSERT_NOT_NULL_MESSAGE(
	    threads[i],
	    "threads[i] == null");
    }
    for (int i = 0; i < THREAD_COUNT; i++) {
	TEST_ASSERT_EQUAL_INT_MESSAGE(
	    WaitForSingleObject(threads[i], INFINITE),
	    0,
	    "WaitForSingleObject() failed");
        TEST_ASSERT_NOT_EQUAL_MESSAGE(
	    CloseHandle(threads[i]),
	    0,
	    "CloseHandle() failed");
        printf("Done: %d / %d\n", i + 1, THREAD_COUNT);
    }
#else
    pthread_t threads[THREAD_COUNT];
    for (int i = 0; i < THREAD_COUNT; i++) {
        TEST_ASSERT_EQUAL_INT_MESSAGE(
            pthread_create(&threads[i], NULL, thread_of_upload_file, &contexts[i]), 0,
            "pthread_create() failed");
    }
    for (int i = 0; i < THREAD_COUNT; i++) {
        TEST_ASSERT_EQUAL_INT_MESSAGE(
            pthread_join(threads[i], NULL), 0,
            "pthread_join() failed");
        printf("Done: %d / %d\n", i + 1, THREAD_COUNT);
    }
#endif
    for (int i = 0; i < THREAD_COUNT; i++) {
        free((void *) contexts[i].key);
    }

    qiniu_ng_upload_token_free(&token);

    DELETE_FILE(file_path);
    free((void *) file_path);

    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_upload_file_path_failed_by_mime(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(
        upload_manager, QINIU_NG_CHARS("z0-bucket"), GETENV(QINIU_NG_CHARS("access_key")), 5);

    qiniu_ng_upload_policy_builder_t policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(QINIU_NG_CHARS("z0-bucket"), config);
    qiniu_ng_upload_token_t token = qiniu_ng_upload_token_new_from_policy_builder(policy_builder, GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));

    qiniu_ng_upload_params_t params = {
        .mime = "invalid"
    };
    qiniu_ng_err_t err;
    qiniu_ng_str_t error;

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_bucket_uploader_upload_file_path(bucket_uploader, token, QINIU_NG_CHARS("/不存在的路径"), &params, NULL, &err),
        "qiniu_ng_bucket_uploader_upload_file_path() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_bad_mime_type_error_extract(&err, &error),
        "qiniu_ng_err_bad_mime_type_error_extract() failed");
    qiniu_ng_str_free(&error);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_err_bad_mime_type_error_extract(&err, &error),
        "qiniu_ng_err_bad_mime_type_error_extract() returns unexpected value");

    qiniu_ng_upload_token_free(&token);

    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_upload_file_path_failed_by_non_existed_path(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(
        upload_manager, QINIU_NG_CHARS("z0-bucket"), GETENV(QINIU_NG_CHARS("access_key")), 5);

    qiniu_ng_upload_policy_builder_t policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(QINIU_NG_CHARS("z0-bucket"), config);
    qiniu_ng_upload_token_t token = qiniu_ng_upload_token_new_from_policy_builder(policy_builder, GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));

    qiniu_ng_err_t err;
    int32_t code;

    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_bucket_uploader_upload_file_path(bucket_uploader, token, QINIU_NG_CHARS("/不存在的路径"), NULL, NULL, &err),
        "qiniu_ng_bucket_uploader_upload_file_path() returns unexpected value");
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_err_bad_mime_type_error_extract(&err, NULL),
        "qiniu_ng_err_bad_mime_type_error_extract() returns unexpected value");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &code),
        "qiniu_ng_err_os_error_extract() failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        strerror(code), "No such file or directory",
        "strerror(code) != \"No such file or directory\"");
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_err_os_error_extract(&err, &code),
        "qiniu_ng_err_os_error_extract() returns unexpected value");

    qiniu_ng_upload_token_free(&token);

    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
}
