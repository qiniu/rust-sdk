#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_bucket_get_name(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);

    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));
    qiniu_ng_str_t bucket_name = qiniu_ng_bucket_get_name(bucket);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(bucket_name), QINIU_NG_CHARS("z0-bucket"));
    qiniu_ng_str_free(bucket_name);
    qiniu_ng_bucket_free(bucket);

    qiniu_ng_bucket_t bucket_2 = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z1-bucket"));
    qiniu_ng_str_t bucket_name_2 = qiniu_ng_bucket_get_name(bucket_2);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(bucket_name_2), QINIU_NG_CHARS("z1-bucket"));
    qiniu_ng_str_free(bucket_name_2);
    qiniu_ng_bucket_free(bucket_2);

    qiniu_ng_client_free(client);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_bucket_get_region(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));

    qiniu_ng_region_t region;
    const qiniu_ng_char_t *io_url;

    TEST_ASSERT_TRUE(qiniu_ng_bucket_get_region(bucket, &region, NULL));
    qiniu_ng_str_list_t io_urls = qiniu_ng_region_get_io_urls(region, false);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(io_urls), 1);
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(io_urls, 0, &io_url));
    TEST_ASSERT_EQUAL_STRING(io_url, QINIU_NG_CHARS("http://iovip.qbox.me"));

    qiniu_ng_str_list_free(io_urls);
    qiniu_ng_region_free(region);
    qiniu_ng_bucket_free(bucket);
    qiniu_ng_client_free(client);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_bucket_get_unexisted_region(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("not-existed-bucket"));

    qiniu_ng_err_t err;
    unsigned short code;
    qiniu_ng_str_t error_message;

    TEST_ASSERT_FALSE(qiniu_ng_bucket_get_region(bucket, NULL, &err));
    TEST_ASSERT_FALSE(qiniu_ng_err_os_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_io_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_json_error_extract(&err, NULL));
    TEST_ASSERT_FALSE(qiniu_ng_err_unknown_error_extract(&err, NULL));
    TEST_ASSERT_TRUE(qiniu_ng_err_response_status_code_error_extract(&err, &code, &error_message));
    TEST_ASSERT_EQUAL_UINT(code, 631);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(error_message), QINIU_NG_CHARS("no such bucket"));
    TEST_ASSERT_FALSE(qiniu_ng_err_response_status_code_error_extract(&err, NULL, NULL));

    qiniu_ng_str_free(error_message);
    qiniu_ng_bucket_free(bucket);
    qiniu_ng_client_free(client);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_bucket_get_regions(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new(client, QINIU_NG_CHARS("z0-bucket"));

    qiniu_ng_regions_t regions;
    qiniu_ng_region_t region;
    qiniu_ng_str_list_t io_urls;
    const qiniu_ng_char_t *io_url;

    TEST_ASSERT_TRUE(qiniu_ng_bucket_get_regions(bucket, &regions, NULL));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_regions_len(regions), 2);

    TEST_ASSERT_TRUE(qiniu_ng_regions_get(regions, 0, &region));
    io_urls = qiniu_ng_region_get_io_urls(region, true);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(io_urls), 1);
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(io_urls, 0, &io_url));
    TEST_ASSERT_EQUAL_STRING(io_url, QINIU_NG_CHARS("https://iovip.qbox.me"));
    qiniu_ng_str_list_free(io_urls);
    qiniu_ng_region_free(region);

    TEST_ASSERT_TRUE(qiniu_ng_regions_get(regions, 1, &region));
    io_urls = qiniu_ng_region_get_io_urls(region, true);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(io_urls), 1);
    TEST_ASSERT_TRUE(qiniu_ng_str_list_get(io_urls, 0, &io_url));
    TEST_ASSERT_EQUAL_STRING(io_url, QINIU_NG_CHARS("https://iovip-z1.qbox.me"));
    qiniu_ng_str_list_free(io_urls);
    qiniu_ng_region_free(region);

    qiniu_ng_regions_free(regions);
    qiniu_ng_bucket_free(bucket);
    qiniu_ng_client_free(client);
    qiniu_ng_config_free(config);
}

void test_qiniu_ng_bucket_new(void) {
    qiniu_ng_config_t config = qiniu_ng_config_new_default();

    env_load("..", false);
    qiniu_ng_client_t client = qiniu_ng_client_new(GETENV(QINIU_NG_CHARS("access_key")), GETENV(QINIU_NG_CHARS("secret_key")), config);

    qiniu_ng_region_t region = qiniu_ng_region_get_region_by_id(qiniu_ng_region_z2);
    const qiniu_ng_char_t* domains_array[2] = {
        QINIU_NG_CHARS("domain1.bucket_z2.com"),
        QINIU_NG_CHARS("domain2.bucket_z2.com")
    };
    qiniu_ng_bucket_t bucket = qiniu_ng_bucket_new2(client, QINIU_NG_CHARS("z2-bucket"), &region, (const qiniu_ng_char_t **) domains_array, 2);

    qiniu_ng_regions_t regions;
    TEST_ASSERT_TRUE(qiniu_ng_bucket_get_regions(bucket, &regions, NULL));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_regions_len(regions), 1);
    TEST_ASSERT_TRUE(qiniu_ng_regions_get(regions, 0, &region));

    qiniu_ng_region_id_t id;
    TEST_ASSERT_TRUE(qiniu_ng_region_get_region_id(region, &id));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_region_id_name(id), "z2");
    qiniu_ng_regions_free(regions);

    qiniu_ng_str_list_t domains;
    const qiniu_ng_char_t *domain = NULL;
    TEST_ASSERT_TRUE(qiniu_ng_bucket_get_domains(bucket, &domains, NULL));
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(domains), 2);
    qiniu_ng_str_list_get(domains, 0, &domain);
    TEST_ASSERT_EQUAL_STRING(domain, domains_array[0]);
    qiniu_ng_str_list_get(domains, 1, &domain);
    TEST_ASSERT_EQUAL_STRING(domain, domains_array[1]);
    qiniu_ng_str_list_free(domains);

    qiniu_ng_bucket_free(bucket);
    qiniu_ng_region_free(region);
    qiniu_ng_client_free(client);
    qiniu_ng_config_free(config);
}
