#ifndef __TEST_H
#define __TEST_H

#include "libqiniu_ng.h"

#if defined(_WIN32) || defined(WIN32)
    #define QINIU_NG_CHARS(x) (L##x)
    #define GETENV(env) (_wgetenv(env))
    #define QINIU_NG_CHARS_LEN(str) (wcslen(str))
    #define QINIU_NG_CHARS_CMP(str1, str2) (wcscmp(str1, str2))
    #define QINIU_NG_CHARS_NCMP(str1, str2, size) (wcsncmp(str1, str2, size))
    #define QINIU_NG_CHARS_STR(str1, str2) (wcsstr(str1, str2))
    #define OPEN_FILE_FOR_READING(file) (_wfopen(file, L"rb"))
    #define OPEN_FILE_FOR_WRITING(file) (_wfopen(file, L"wb"))
    #define DELETE_FILE(file) (_wunlink(file))
#else
    #define QINIU_NG_CHARS(x) x
    #define GETENV(env) (getenv(env))
    #define QINIU_NG_CHARS_LEN(str) (strlen(str))
    #define QINIU_NG_CHARS_CMP(str1, str2) (strcmp(str1, str2))
    #define QINIU_NG_CHARS_NCMP(str1, str2, size) (strncmp(str1, str2, size))
    #define QINIU_NG_CHARS_STR(str1, str2) (strstr(str1, str2))
    #define OPEN_FILE_FOR_READING(file) (fopen(file, "r"))
    #define OPEN_FILE_FOR_WRITING(file) (fopen(file, "w"))
    #define DELETE_FILE(file) (unlink(file))
#endif

int env_load(char*, bool);
void write_str_to_file(const qiniu_ng_char_t* path, const char* content);
qiniu_ng_char_t* create_temp_file(size_t size);

void test_qiniu_ng_etag_from_file_path(void);
void test_qiniu_ng_etag_from_buffer(void);
void test_qiniu_ng_etag_from_unexisted_file_path(void);
void test_qiniu_ng_etag_from_large_buffer(void);
void test_qiniu_ng_config_new_default(void);
void test_qiniu_ng_config_new(void);
void test_qiniu_ng_config_new2(void);
void test_qiniu_ng_config_http_request_handlers(void);
void test_qiniu_ng_config_bad_http_request_handlers(void);
void test_qiniu_ng_config_bad_http_request_handlers_2(void);
void test_qiniu_ng_region_query(void);
void test_qiniu_ng_region_get_by_id(void);
void test_qiniu_ng_storage_bucket_names(void);
void test_qiniu_ng_storage_bucket_create_and_drop(void);
void test_qiniu_ng_storage_bucket_create_duplicated(void);
void test_qiniu_ng_bucket_get_name(void);
void test_qiniu_ng_bucket_get_region(void);
void test_qiniu_ng_bucket_get_unexisted_region(void);
void test_qiniu_ng_bucket_get_regions(void);
void test_qiniu_ng_bucket_builder(void);
void test_qiniu_ng_bucket_get_regions_and_domains(void);
void test_qiniu_ng_make_upload_token(void);
void test_qiniu_ng_upload_files(void);
void test_qiniu_ng_upload_huge_number_of_files(void);
void test_qiniu_ng_upload_file_path_failed_by_mime(void);
void test_qiniu_ng_upload_file_path_failed_by_non_existed_path(void);
void test_qiniu_ng_str(void);
void test_qiniu_ng_str_list(void);
void test_qiniu_ng_str_map(void);

#endif
