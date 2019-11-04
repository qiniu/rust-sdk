#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_bucket_name(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);

    qiniu_ng_bucket_t bucket = qiniu_ng_bucket(client, "z0-bucket");
    qiniu_ng_string_t bucket_name = qiniu_ng_bucket_name(bucket);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(bucket_name), "z0-bucket");
    qiniu_ng_string_free(bucket_name);
    qiniu_ng_bucket_free(bucket);

    qiniu_ng_bucket_t bucket_2 = qiniu_ng_bucket(client, "z1-bucket");
    qiniu_ng_string_t bucket_name_2 = qiniu_ng_bucket_name(bucket_2);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(bucket_name_2), "z1-bucket");
    qiniu_ng_string_free(bucket_name_2);
    qiniu_ng_bucket_free(bucket_2);

    qiniu_ng_client_free(client);
}

void test_qiniu_ng_bucket_region(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket(client, "z0-bucket");

    qiniu_ng_region_t region;
    qiniu_ng_err err;

    TEST_ASSERT_TRUE(qiniu_ng_bucket_region(bucket, &region, &err));
    qiniu_ng_string_t rs_url = qiniu_ng_region_get_rs_url(region, false);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "http://rs.qiniu.com");

    qiniu_ng_string_free(rs_url);
    qiniu_ng_region_free(region);
    qiniu_ng_bucket_free(bucket);
    qiniu_ng_client_free(client);
}

void test_qiniu_ng_bucket_regions(void) {
    qiniu_ng_config_t config;
    qiniu_ng_config_init(&config);

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(getenv("access_key"), getenv("secret_key"), &config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket(client, "z0-bucket");

    qiniu_ng_regions_t regions;
    qiniu_ng_region_t region;
    qiniu_ng_string_t rs_url;
    qiniu_ng_err err;

    TEST_ASSERT_TRUE(qiniu_ng_bucket_regions(bucket, &regions, &err));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_regions_len(regions), 2);

    TEST_ASSERT_TRUE(qiniu_ng_regions_get(regions, 0, &region));
    rs_url = qiniu_ng_region_get_rs_url(region, false);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "http://rs.qiniu.com");
    qiniu_ng_string_free(rs_url);
    qiniu_ng_region_free(region);

    TEST_ASSERT_TRUE(qiniu_ng_regions_get(regions, 1, &region));
    rs_url = qiniu_ng_region_get_rs_url(region, false);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(rs_url), "http://rs-z1.qiniu.com");
    qiniu_ng_string_free(rs_url);
    qiniu_ng_region_free(region);

    qiniu_ng_regions_free(regions);
    qiniu_ng_bucket_free(bucket);
    qiniu_ng_client_free(client);
}
