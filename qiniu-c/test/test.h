#include "libqiniu_ng.h"

int env_load(char*, bool);
void write_str_to_file(const char* path, const char* content);

void test_qiniu_ng_etag_from_file_path(void);
void test_qiniu_ng_etag_from_buffer(void);
void test_qiniu_ng_etag_from_unexisted_file_path(void);
void test_qiniu_ng_etag_from_large_buffer(void);
void test_qiniu_ng_config(void);
void test_qiniu_ng_region_query(void);
void test_qiniu_ng_storage_bucket_names(void);
void test_qiniu_ng_storage_bucket_create_and_drop(void);
void test_qiniu_ng_bucket_name(void);
void test_qiniu_ng_bucket_region(void);
void test_qiniu_ng_bucket_regions(void);
void test_qiniu_ng_make_upload_token(void);
