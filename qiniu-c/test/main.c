#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void setUp(void) {

}

void tearDown(void) {

}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_qiniu_ng_string);
    RUN_TEST(test_qiniu_ng_str_list);
    RUN_TEST(test_qiniu_ng_string_list);
    RUN_TEST(test_qiniu_ng_str_map);
    RUN_TEST(test_qiniu_ng_etag_from_file_path);
    RUN_TEST(test_qiniu_ng_etag_from_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_large_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_unexisted_file_path);
    RUN_TEST(test_qiniu_ng_config_new_default);
    RUN_TEST(test_qiniu_ng_config_new);
    RUN_TEST(test_qiniu_ng_config_new2);
    RUN_TEST(test_qiniu_ng_region_query);
    RUN_TEST(test_qiniu_ng_region_get_by_id);
    RUN_TEST(test_qiniu_ng_storage_bucket_names);
    RUN_TEST(test_qiniu_ng_storage_bucket_create_and_drop);
    RUN_TEST(test_qiniu_ng_bucket_get_name);
    RUN_TEST(test_qiniu_ng_bucket_get_region);
    RUN_TEST(test_qiniu_ng_bucket_get_regions);
    RUN_TEST(test_qiniu_ng_bucket_new);
    RUN_TEST(test_qiniu_ng_make_upload_token);
    RUN_TEST(test_qiniu_ng_upload_files);
    RUN_TEST(test_qiniu_ng_upload_file_path_failed_by_mime);
    return UNITY_END();
}

