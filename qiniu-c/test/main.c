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
    return UNITY_END();
}

