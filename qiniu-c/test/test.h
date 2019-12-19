#include "libqiniu_ng.h"

int env_load(char*, bool);
void write_str_to_file(const char* path, const char* content);
char* create_temp_file(size_t size);

void test_qiniu_ng_etag_from_file_path(void);
void test_qiniu_ng_etag_from_buffer(void);
void test_qiniu_ng_etag_from_unexisted_file_path(void);
void test_qiniu_ng_etag_from_large_buffer(void);
void test_qiniu_ng_config_new_default(void);
void test_qiniu_ng_config_new(void);
void test_qiniu_ng_config_new2(void);
void test_qiniu_ng_region_query(void);
void test_qiniu_ng_region_get_by_id(void);
void test_qiniu_ng_storage_bucket_names(void);
void test_qiniu_ng_storage_bucket_create_and_drop(void);
void test_qiniu_ng_bucket_get_name(void);
void test_qiniu_ng_bucket_get_region(void);
void test_qiniu_ng_bucket_get_regions(void);
void test_qiniu_ng_bucket_new(void);
void test_qiniu_ng_make_upload_token(void);
void test_qiniu_ng_upload_files(void);
void test_qiniu_ng_upload_file_path_failed_by_mime(void);
void test_qiniu_ng_string(void);
void test_qiniu_ng_string_list(void);
void test_qiniu_ng_string_map(void);
