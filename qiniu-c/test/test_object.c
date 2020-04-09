#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

static void generate_file_key(const qiniu_ng_char_t *file_key, int max_size, int file_id, int file_size) {
#if defined(_WIN32) || defined(WIN32)
    swprintf((wchar_t *) file_key, max_size, L"测试-%dk-%d-%lld-%d", file_size, file_id, (long long) time(NULL), rand());
#else
    snprintf((char *) file_key, max_size, "测试-%dk-%d-%lld-%d", file_size, file_id, (long long) time(NULL), rand());
#endif
}

void test_qiniu_ng_object_upload_files(void) {
    env_load("..", false);
    const qiniu_ng_char_t file_key[256];
    generate_file_key(file_key, 256, 0, 1);

    const qiniu_ng_char_t *file_path = create_temp_file(1024);
    char etag[ETAG_SIZE + 1];
    memset(&etag, 0, (ETAG_SIZE + 1) * sizeof(char));
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_etag_from_file_path(file_path, (char *) &etag[0], NULL),
        "qiniu_ng_etag_from_file_path() failed");

    qiniu_ng_client_t client = qiniu_ng_client_new_default(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")));
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));
    qiniu_ng_object_t object = qiniu_ng_object_new(bucket, file_key);
    qiniu_ng_upload_response_t upload_response;
    qiniu_ng_err_t err;
    if (!qiniu_ng_object_upload_file_path(object, file_path, NULL, &upload_response, &err)) {
        qiniu_ng_err_fputs(err, stderr);
        TEST_FAIL_MESSAGE("qiniu_ng_object_upload_file_path() failed");
    }

    qiniu_ng_str_t key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE_MESSAGE(qiniu_ng_str_is_null(key), "qiniu_ng_str_is_null(key) != false");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(file_key, qiniu_ng_str_get_ptr(key), "object.key != key");
    qiniu_ng_str_free(&key);

    char hash[ETAG_SIZE + 1];
    size_t hash_size;
    memset(hash, 0, ETAG_SIZE + 1);
    qiniu_ng_upload_response_get_hash(upload_response, (char *) &hash[0], &hash_size);
    TEST_ASSERT_EQUAL_INT_MESSAGE(hash_size, ETAG_SIZE, "hash_size != ETAG_SIZE");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(hash, (const char *) &etag, "hash != etag");

    qiniu_ng_upload_response_free(&upload_response);
    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_object_delete(object, NULL), "qiniu_ng_object_delete() failed");

    FILE *file = OPEN_FILE_FOR_READING(file_path);
    TEST_ASSERT_NOT_NULL_MESSAGE(file, "file == null");
    if (!qiniu_ng_object_upload_file(object, file, NULL, &upload_response, &err)) {
        qiniu_ng_err_fputs(err, stderr);
        TEST_FAIL_MESSAGE("qiniu_ng_object_upload_file() failed");
    }
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        fclose(file), 0,
        "fclose(file) != 0");

    key = qiniu_ng_upload_response_get_key(upload_response);
    TEST_ASSERT_FALSE_MESSAGE(
        qiniu_ng_str_is_null(key),
        "qiniu_ng_str_is_null(key) != false");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(file_key, qiniu_ng_str_get_ptr(key), "object.key != key");
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

    TEST_ASSERT_TRUE_MESSAGE(qiniu_ng_object_delete(object, NULL), "qiniu_ng_object_delete() failed");
    qiniu_ng_object_free(&object);

    qiniu_ng_bucket_free(&bucket);
    qiniu_ng_client_free(&client);
    DELETE_FILE(file_path);
    free((void *) file_path);
}
