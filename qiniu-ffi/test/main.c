#include "unity.h"
#include "test.h"

#if defined(_WIN32) || defined(WIN32)
#pragma comment(lib, "qiniu_ffi.dll.lib")
#endif

void setUp(void) {
    env_load("..", false);
}

void tearDown(void) {

}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_qiniu_ng_etag_v1);
    RUN_TEST(test_qiniu_ng_etag_v2);
    RUN_TEST(test_qiniu_ng_etag_from_file);
    return UNITY_END();
}
