#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void setUp(void) {

}

void tearDown(void) {

}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_qiniu_ng_etag_from_file_path);
    RUN_TEST(test_qiniu_ng_etag_from_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_large_buffer);
    RUN_TEST(test_qiniu_ng_etag_from_unexisted_file_path);
    RUN_TEST(test_qiniu_ng_config);
    RUN_TEST(test_qiniu_ng_region_query);
    RUN_TEST(test_qiniu_ng_storage_bucket_names);
    RUN_TEST(test_qiniu_ng_storage_bucket_test);
    return UNITY_END();
}

