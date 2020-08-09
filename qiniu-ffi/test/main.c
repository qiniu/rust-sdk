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
    RUN_TEST(test_qiniu_ng_str_new);
    RUN_TEST(test_qiniu_ng_str_push_cstr);
    RUN_TEST(test_qiniu_ng_http_headers_get_put);
    RUN_TEST(test_qiniu_ng_credential_get);
    RUN_TEST(test_qiniu_ng_credential_sign);
    RUN_TEST(test_qiniu_ng_credential_sign_with_data);
    RUN_TEST(test_qiniu_ng_credential_authorization_v1);
    RUN_TEST(test_qiniu_ng_credential_authorization_v2);
    RUN_TEST(test_qiniu_ng_credential_sign_download_url);
    RUN_TEST(test_qiniu_ng_credential_provider_static);
    RUN_TEST(test_qiniu_ng_credential_provider_global);
    RUN_TEST(test_qiniu_ng_credential_provider_env);
    RUN_TEST(test_qiniu_ng_credential_provider_chain);
    RUN_TEST(test_qiniu_ng_credential_provider_user_defined);
    RUN_TEST(test_qiniu_ng_etag_v1);
    RUN_TEST(test_qiniu_ng_etag_v2);
    RUN_TEST(test_qiniu_ng_etag_from_file);
    return UNITY_END();
}
