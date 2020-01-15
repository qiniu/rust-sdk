#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_str(void) {
    qiniu_ng_char_t *str = QINIU_NG_CHARS("你好，世界");
    size_t len = QINIU_NG_CHARS_LEN(str);
    qiniu_ng_str_t qiniu_str = qiniu_ng_str_new(str);
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_get_ptr(qiniu_str), str,
        "qiniu_ng_str_get_ptr(qiniu_str) != \"你好，世界\"");
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_str_get_len(qiniu_str), len,
        "qiniu_ng_str_get_len(qiniu_str) != len(\"你好，世界\")");
    qiniu_ng_str_free(&qiniu_str);
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_str_is_freed(qiniu_str),
        "qiniu_ng_str_is_freed() failed");
    TEST_ASSERT_NULL_MESSAGE(
        qiniu_str._0,
        "qiniu_str._0 != null");
    TEST_ASSERT_NULL_MESSAGE(
        qiniu_str._1,
        "qiniu_str._1 != null");
    qiniu_ng_str_free(&qiniu_str);
}

void test_qiniu_ng_str_list(void) {
    const qiniu_ng_char_t *strlist[3] = {QINIU_NG_CHARS("你好，世界"), QINIU_NG_CHARS("你好，七牛"), QINIU_NG_CHARS("你好，科多兽")};
    qiniu_ng_str_list_t list = qiniu_ng_str_list_new((const qiniu_ng_char_t* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_str_list_len(list), 3,
        "qiniu_ng_str_list_len(list) != 3");

    const qiniu_ng_char_t *str;
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_str_list_get(list, 0, &str),
        "qiniu_ng_str_list_get(list, 0, &str) failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        str, QINIU_NG_CHARS("你好，世界"),
        "str != \"你好，世界\"");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_str_list_get(list, 1, &str),
        "qiniu_ng_str_list_get(list, 1, &str) failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        str, QINIU_NG_CHARS("你好，七牛"),
        "str != \"你好，七牛\"");
    TEST_ASSERT_TRUE_MESSAGE(
        qiniu_ng_str_list_get(list, 2, &str),
        "qiniu_ng_str_list_get(list, 2, &str) failed");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        str, QINIU_NG_CHARS("你好，科多兽"),
        "str != \"你好，科多兽\"");

    qiniu_ng_str_list_free(&list);
}

static int test_qiniu_ng_str_map_handler(const qiniu_ng_char_t*, const qiniu_ng_char_t*, void*);
void test_qiniu_ng_str_map(void) {
    qiniu_ng_str_map_t map = qiniu_ng_str_map_new(5);
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("qiniu"), QINIU_NG_CHARS("七牛"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("kodo"), QINIU_NG_CHARS("科多兽"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("dora"), QINIU_NG_CHARS("多啦A梦"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("pandora"), QINIU_NG_CHARS("潘多拉"));

    TEST_ASSERT_EQUAL_INT_MESSAGE(
        qiniu_ng_str_map_len(map), 4,
        "qiniu_ng_str_map_len(map) != 4");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(map, QINIU_NG_CHARS("dora")), QINIU_NG_CHARS("多啦A梦"),
        "qiniu_ng_str_map_get(map, \"dora\") != \"多啦A梦\"");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(map, QINIU_NG_CHARS("qiniu")), QINIU_NG_CHARS("七牛"),
        "qiniu_ng_str_map_get(map, \"qiniu\") != \"七牛\"");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(map, QINIU_NG_CHARS("kodo")), QINIU_NG_CHARS("科多兽"),
        "qiniu_ng_str_map_get(map, \"kodo\") != \"科多兽\"");
    TEST_ASSERT_EQUAL_STRING_MESSAGE(
        qiniu_ng_str_map_get(map, QINIU_NG_CHARS("pandora")), QINIU_NG_CHARS("潘多拉"),
        "qiniu_ng_str_map_get(map, \"pandora\") != \"潘多拉\"");

    unsigned long score = 0;
    qiniu_ng_str_map_each_entry(map, test_qiniu_ng_str_map_handler, &score);
    TEST_ASSERT_EQUAL_INT_MESSAGE(
        score, 10,
        "score != 10");
    qiniu_ng_str_map_free(&map);
}

static int test_qiniu_ng_str_map_handler(const qiniu_ng_char_t *key, const qiniu_ng_char_t *value, void *score) {
    if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("qiniu")) == 0) {
        TEST_ASSERT_EQUAL_STRING_MESSAGE(
            value, QINIU_NG_CHARS("七牛"),
            "value != \"七牛\"");
        (*(unsigned long *) score) += 1;
        return 0;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("kodo")) == 0) {
        TEST_ASSERT_EQUAL_STRING_MESSAGE(
            value, QINIU_NG_CHARS("科多兽"),
            "value != \"科多兽\"");
        (*(unsigned long *) score) += 2;
        return 0;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("dora")) == 0) {
        TEST_ASSERT_EQUAL_STRING_MESSAGE(
            value, QINIU_NG_CHARS("多啦A梦"),
            "value != \"多啦A梦\"");
        (*(unsigned long *) score) += 3;
        return 0;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("pandora")) == 0) {
        TEST_ASSERT_EQUAL_STRING_MESSAGE(
            value, QINIU_NG_CHARS("潘多拉"),
            "value != \"潘多拉\"");
        (*(unsigned long *) score) += 4;
        return 0;
    } else {
        return -1;
    }
}
