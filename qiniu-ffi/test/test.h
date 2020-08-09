#ifndef __TEST_H
#define __TEST_H

#include "libqiniu_ng.h"

// Utilties
int env_load(char*, bool);

// Test Cases

// Str
void test_qiniu_ng_str_new(void);
void test_qiniu_ng_str_push_cstr(void);

// HTTP
void test_qiniu_ng_http_headers_get_put(void);

// Credential
void test_qiniu_ng_credential_get(void);
void test_qiniu_ng_credential_sign(void);
void test_qiniu_ng_credential_sign_with_data(void);
void test_qiniu_ng_credential_authorization_v1(void);
void test_qiniu_ng_credential_authorization_v2(void);
void test_qiniu_ng_credential_sign_download_url(void);
// Credential Provider
void test_qiniu_ng_credential_provider_static(void);
void test_qiniu_ng_credential_provider_global(void);
void test_qiniu_ng_credential_provider_env(void);
void test_qiniu_ng_credential_provider_chain(void);
void test_qiniu_ng_credential_provider_user_defined(void);

// Etag
void test_qiniu_ng_etag_v1(void);
void test_qiniu_ng_etag_v2(void);
void test_qiniu_ng_etag_from_file(void);

#endif
