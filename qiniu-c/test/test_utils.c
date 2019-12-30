#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_str(void) {
    const char* str = "hello world";
    qiniu_ng_str_t qiniu_str = qiniu_ng_str_new(str);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(qiniu_str), str);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_get_len(qiniu_str), strlen(str));
    qiniu_ng_str_free(qiniu_str);
}

void test_qiniu_ng_string(void) {
#if defined(_WIN32) || defined(WIN32)
    const wchar_t* str = L"你好，世界";
    size_t len = wcslen(str);
#else
    const char* str = "你好，世界";
    size_t len = strlen(str);
#endif
    qiniu_ng_string_t qiniu_str = qiniu_ng_string_new(str);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_string_get_ptr(qiniu_str), str);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_get_len(qiniu_str), len);
    qiniu_ng_string_free(qiniu_str);
}

void test_qiniu_ng_str_list(void) {
    const char *strlist[3] = {"hello world", "hello qiniu", "hello kodo"};

    qiniu_ng_str_list_t list = qiniu_ng_str_list_new((const char* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(list), 3);

    const char *str;
    qiniu_ng_str_list_get(list, 0, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello world");
    qiniu_ng_str_list_get(list, 1, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello qiniu");
    qiniu_ng_str_list_get(list, 2, &str);
    TEST_ASSERT_EQUAL_STRING(str, "hello kodo");

    qiniu_ng_str_list_free(list);
}

void test_qiniu_ng_string_list(void) {
#if defined(_WIN32) || defined(WIN32)
    const wchar_t *strlist[3] = {L"你好，世界", L"你好，七牛", L"你好，科多兽"};
#else
    const char *strlist[3] = {"你好，世界", "你好，七牛", "你好，科多兽"};
#endif

    qiniu_ng_string_list_t list = qiniu_ng_string_list_new((const qiniu_ng_char_t* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_string_list_len(list), 3);

    const qiniu_ng_char_t *str;
    qiniu_ng_string_list_get(list, 0, &str);
#if defined(_WIN32) || defined(WIN32)
    TEST_ASSERT_EQUAL_STRING(str, L"你好，世界");
#else
    TEST_ASSERT_EQUAL_STRING(str, "你好，世界");
#endif
    qiniu_ng_string_list_get(list, 1, &str);
#if defined(_WIN32) || defined(WIN32)
    TEST_ASSERT_EQUAL_STRING(str, L"你好，七牛");
#else
    TEST_ASSERT_EQUAL_STRING(str, "你好，七牛");
#endif
    qiniu_ng_string_list_get(list, 2, &str);
#if defined(_WIN32) || defined(WIN32)
    TEST_ASSERT_EQUAL_STRING(str, L"你好，科多兽");
#else
    TEST_ASSERT_EQUAL_STRING(str, "你好，科多兽");
#endif

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
