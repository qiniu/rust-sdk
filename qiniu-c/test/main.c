#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

#if defined(_WIN32) || defined(WIN32)
#pragma comment(lib, "qiniu_ng_c.dll.lib")
#endif

void setUp(void) {

}

void tearDown(void) {

}

int main(void) {
    printf("Version = %s, Features = %s\n", qiniu_ng_version(), qiniu_ng_features());
    UNITY_BEGIN();
    RUN_TEST(test_qiniu_ng_str);
    RUN_TEST(test_qiniu_ng_str_list);
    RUN_TEST(test_qiniu_ng_str_map);
    RUN_TEST(test_qiniu_ng_etag_from_file_path);
    RUN_TEST(test_qiniu_ng_etag_from_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_large_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_unexisted_file_path);
    RUN_TEST(test_qiniu_ng_config_new_default);
    RUN_TEST(test_qiniu_ng_config_new);
    RUN_TEST(test_qiniu_ng_config_new2);
    RUN_TEST(test_qiniu_ng_config_http_request_handlers);
    RUN_TEST(test_qiniu_ng_config_bad_http_request_handlers);
    RUN_TEST(test_qiniu_ng_config_bad_http_request_handlers_2);
    RUN_TEST(test_qiniu_ng_region_query);
    RUN_TEST(test_qiniu_ng_region_get_by_id);
    RUN_TEST(test_qiniu_ng_storage_bucket_names);
    RUN_TEST(test_qiniu_ng_storage_bucket_create_and_drop);
    RUN_TEST(test_qiniu_ng_storage_bucket_create_duplicated);
    RUN_TEST(test_qiniu_ng_bucket_get_name);
    RUN_TEST(test_qiniu_ng_bucket_get_region);
    RUN_TEST(test_qiniu_ng_bucket_get_unexisted_region);
    RUN_TEST(test_qiniu_ng_bucket_get_regions);
    RUN_TEST(test_qiniu_ng_bucket_builder);
    RUN_TEST(test_qiniu_ng_bucket_get_regions_and_domains);
    RUN_TEST(test_qiniu_ng_make_upload_token);
    RUN_TEST(test_qiniu_ng_upload_file_path_failed_by_mime);
    RUN_TEST(test_qiniu_ng_upload_file_path_failed_by_non_existed_path);
    RUN_TEST(test_qiniu_ng_upload_files);
    RUN_TEST(test_qiniu_ng_upload_huge_number_of_files);
    return UNITY_END();
}

