#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config(void) {
    void* config_builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_use_https(config_builder);
    qiniu_ng_config_builder_batch_max_operation_size(config_builder, 500);
    qiniu_ng_config_builder_host_freeze_duration(config_builder, 60);
    qiniu_ng_config_builder_upload_token_lifetime(config_builder, 7200);
    void* config = qiniu_ng_config_builder_build(config_builder);
    TEST_ASSERT_TRUE(qiniu_ng_config_use_https(config));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_batch_max_operation_size(config), 500);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_host_freeze_duration(config), 60);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_upload_token_lifetime(config), 7200);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_config_upload_chunk_size(config), 1 << 22);
    qiniu_ng_config_free(config);
}
