#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config(void) {
    qiniu_ng_config config;
    qiniu_ng_config_init(&config);

    TEST_ASSERT_FALSE(config.use_https);
    TEST_ASSERT_EQUAL_INT(config.batch_max_operation_size, 1000);
    TEST_ASSERT_EQUAL_INT(config.upload_threshold, 1 << 22);
}
