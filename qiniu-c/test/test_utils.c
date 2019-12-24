#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_string(void) {
    const char* str = "hello world";
    qiniu_ng_string_t qiniu_str = qiniu_ng_string_new(str);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(qiniu_str), str);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_get_len(qiniu_str), strlen(str));
    qiniu_ng_string_free(qiniu_str);

    qiniu_str = qiniu_ng_string_new_with_len(str, 5);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(qiniu_str), "hello");
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_get_len(qiniu_str), 5);
    qiniu_ng_string_free(qiniu_str);
}

void test_qiniu_ng_str_list(void) {
    const char *strlist[3] = {"hello world", "hello qiniu", "hello kodo"};

    qiniu_ng_string_list_t list = qiniu_ng_string_list_new((const char* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_list_len(list), 3);

    const char *str;
    qiniu_ng_string_list_get(list, 0, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello world");
    qiniu_ng_string_list_get(list, 1, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello qiniu");
    qiniu_ng_string_list_get(list, 2, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello kodo");

    qiniu_ng_string_list_free(list);
}

void test_qiniu_ng_string_list(void) {
    const qiniu_ng_char_t *strlist[3] = {"hello world", "hello qiniu", "hello kodo"};

    qiniu_ng_string_list_t list = qiniu_ng_string_list_new((const qiniu_ng_char_t* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_list_len(list), 3);

    const qiniu_ng_char_t *str;
    qiniu_ng_string_list_get(list, 0, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello world");
    qiniu_ng_string_list_get(list, 1, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello qiniu");
    qiniu_ng_string_list_get(list, 2, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello kodo");

    qiniu_ng_string_list_free(list);
}

bool test_qiniu_ng_str_map_handler(const char*, const char*, void*);
void test_qiniu_ng_str_map(void) {
    qiniu_ng_str_map_t map = qiniu_ng_str_map_new(5);
    qiniu_ng_str_map_set(map, "qiniu", "Qiniu");
    qiniu_ng_str_map_set(map, "kodo", "KODO");
    qiniu_ng_str_map_set(map, "dora", "Dora");
    qiniu_ng_str_map_set(map, "pandora", "Pandora");

    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_map_len(map), 4);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, "dora"), "Dora");
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, "qiniu"), "Qiniu");

    int score = 0;
    qiniu_ng_str_map_each_entry(map, test_qiniu_ng_str_map_handler, &score);
    TEST_ASSERT_EQUAL_INT(score, 10);
    qiniu_ng_str_map_free(map);
}

bool test_qiniu_ng_str_map_handler(const char *key, const char *value, void *score) {
    if (strcmp(key, "qiniu") == 0) {
        TEST_ASSERT_EQUAL_STRING(value, "Qiniu");
        (*(int *) score) += 1;
        return true;
    } else if (strcmp(key, "kodo") == 0) {
        TEST_ASSERT_EQUAL_STRING(value, "KODO");
        (*(int *) score) += 2;
        return true;
    } else if (strcmp(key, "dora") == 0) {
        TEST_ASSERT_EQUAL_STRING(value, "Dora");
        (*(int *) score) += 3;
        return true;
    } else if (strcmp(key, "pandora") == 0) {
        TEST_ASSERT_EQUAL_STRING(value, "Pandora");
        (*(int *) score) += 4;
        return true;
    } else {
        return false;
    }
}
