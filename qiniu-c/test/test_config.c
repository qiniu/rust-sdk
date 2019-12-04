#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_config(void) {
    qiniu_ng_config_fields_t fields, fields2;
    qiniu_ng_config_fields_init(&fields);

    TEST_ASSERT_FALSE(fields.use_https);
    TEST_ASSERT_EQUAL_INT(fields.batch_max_operation_size, 1000);
    TEST_ASSERT_EQUAL_INT(fields.upload_threshold, 1 << 22);

    fields.use_https = true;
    fields.batch_max_operation_size = 10000;

    qiniu_ng_config_t config = qiniu_ng_config_new(&fields);
    qiniu_ng_config_fill_fields(config, &fields2);

    TEST_ASSERT_TRUE(fields2.use_https);
    TEST_ASSERT_EQUAL_INT(fields2.batch_max_operation_size, 10000);
    TEST_ASSERT_EQUAL_INT(fields2.upload_threshold, 1 << 22);

    qiniu_ng_config_free(config);
}
