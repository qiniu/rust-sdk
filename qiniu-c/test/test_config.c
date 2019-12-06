#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config(void) {
    qiniu_ng_config_fields_t fields;
    qiniu_ng_config_fields_init(&fields);

    TEST_ASSERT_FALSE(fields.use_https);
    TEST_ASSERT_EQUAL_INT(fields.batch_max_operation_size, 1000);
    TEST_ASSERT_EQUAL_INT(fields.upload_threshold, 1 << 22);

    fields.use_https = true;
    fields.batch_max_operation_size = 10000;
    fields.rs_host = "rspub.qiniu.com";

    qiniu_ng_config_t config = qiniu_ng_config_new(&fields);

    TEST_ASSERT_TRUE(qiniu_ng_config_get_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_batch_max_operation_size(config), 10000);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_get_upload_threshold(config), 1 << 22);

    qiniu_ng_string_t rs_url = qiniu_ng_config_get_rs_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "https://rspub.qiniu.com");
    qiniu_ng_string_free(rs_url);

    qiniu_ng_string_t uc_url = qiniu_ng_config_get_uc_url(config);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(uc_url), "https://uc.qbox.me");
    qiniu_ng_string_free(uc_url);

    qiniu_ng_config_free(config);
}
