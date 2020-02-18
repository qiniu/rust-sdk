#include "unity.h"
#include "libqiniu_ng.h"
#include "test.h"

void test_qiniu_ng_region_query(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_regions_t regions;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_region_query(QINIU_NG_CHARS("z0-bucket"), GETENV(QINIU_NG_CHARS("access_key")), config, &regions, NULL),
        "qiniu_ng_region_query() failed");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_regions_len(regions), 2,
        "qiniu_ng_regions_len(regions) != 2");

    qiniu_ng_region_t region;
    qiniu_ng_str_list_t urls;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_regions_get(regions, 0, &region),
        "qiniu_ng_regions_get(regions, 0, &region) failed");
    urls = qiniu_ng_region_get_up_urls(region, true);
    size_t urls_len = qiniu_ng_str_list_len(urls);
    TEST_ASSERT_GREATER_THAN_MESSAGE(
        4, urls_len,
        "urls_len <= 4");

    for (size_t i = 0; i < urls_len; i++) {
        const qiniu_ng_char_t* p = qiniu_ng_str_list_get(urls, i);
        TEST_ASSERT_NOT_NULL_MESSAGE(
            p,
            "p == null");
    }

    qiniu_ng_str_list_free(&urls);
    qiniu_ng_region_free(&region);

    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_regions_get(regions, 1, &region),
        "qiniu_ng_regions_get(regions, 1, &region) failed");
    urls = qiniu_ng_region_get_io_urls(region, true);
    urls_len = qiniu_ng_str_list_len(urls);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        urls_len, 1,
        "urls_len != 1");
    for (size_t i = 0; i < urls_len; i++) {
        const qiniu_ng_char_t* p = qiniu_ng_str_list_get(urls, i);
        TEST_ASSERT_NOT_NULL_MESSAGE(
            p,
            "p == null");
    }
    qiniu_ng_region_free(&region);

    qiniu_ng_regions_free(&regions);
    qiniu_ng_config_free(&config);
}

void test_qiniu_ng_region_get_by_id(void) {
    qiniu_ng_region_id_t id;
    qiniu_ng_region_t region = qiniu_ng_region_get_by_id(qiniu_ng_region_z0);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_region_get_region_id(region, &id),
        "qiniu_ng_region_get_region_id() failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_region_id_name(id), "z0",
        "qiniu_ng_region_id_name(id) != \"z0\"");
    qiniu_ng_region_free(&region);

    region = qiniu_ng_region_get_by_id(qiniu_ng_region_na0);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_region_get_region_id(region, &id),
        "qiniu_ng_region_get_region_id() failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_region_id_name(id), "na0",
        "qiniu_ng_region_id_name(id) != \"na0\"");
    qiniu_ng_region_free(&region);
}
