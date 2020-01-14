#include "unity.h"
#include "libqiniu_ng.h"
#include <string.h>
#include "test.h"

void test_qiniu_ng_str(void) {
    qiniu_ng_char_t *str = QINIU_NG_CHARS("你好，世界");
    size_t len = QINIU_NG_CHARS_LEN(str);
    qiniu_ng_str_t qiniu_str = qiniu_ng_str_new(str);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_get_ptr(qiniu_str), str);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_get_len(qiniu_str), len);
    qiniu_ng_str_free(&qiniu_str);
    TEST_ASSERT_TRUE(qiniu_ng_str_is_freed(qiniu_str));
    TEST_ASSERT_NULL(qiniu_str._0);
    TEST_ASSERT_NULL(qiniu_str._1);
    qiniu_ng_str_free(&qiniu_str);
}

void test_qiniu_ng_str_list(void) {
    const qiniu_ng_char_t *strlist[3] = {QINIU_NG_CHARS("你好，世界"), QINIU_NG_CHARS("你好，七牛"), QINIU_NG_CHARS("你好，科多兽")};
    qiniu_ng_str_list_t list = qiniu_ng_str_list_new((const qiniu_ng_char_t* const*) strlist, 3);
    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_list_len(list), 3);

    const qiniu_ng_char_t *str;
    qiniu_ng_str_list_get(list, 0, &str);
    TEST_ASSERT_EQUAL_STRING(str, QINIU_NG_CHARS("你好，世界"));
    qiniu_ng_str_list_get(list, 1, &str);
    TEST_ASSERT_EQUAL_STRING(str, QINIU_NG_CHARS("你好，七牛"));
    qiniu_ng_str_list_get(list, 2, &str);
    TEST_ASSERT_EQUAL_STRING(str, QINIU_NG_CHARS("你好，科多兽"));

    qiniu_ng_str_list_free(&list);
}

bool test_qiniu_ng_str_map_handler(const qiniu_ng_char_t*, const qiniu_ng_char_t*, void*);
void test_qiniu_ng_str_map(void) {
    qiniu_ng_str_map_t map = qiniu_ng_str_map_new(5);
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("qiniu"), QINIU_NG_CHARS("七牛"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("kodo"), QINIU_NG_CHARS("科多兽"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("dora"), QINIU_NG_CHARS("多啦A梦"));
    qiniu_ng_str_map_set(map, QINIU_NG_CHARS("pandora"), QINIU_NG_CHARS("潘多拉"));

    TEST_ASSERT_EQUAL_INT(qiniu_ng_str_map_len(map), 4);
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, QINIU_NG_CHARS("dora")), QINIU_NG_CHARS("多啦A梦"));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, QINIU_NG_CHARS("qiniu")), QINIU_NG_CHARS("七牛"));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, QINIU_NG_CHARS("kodo")), QINIU_NG_CHARS("科多兽"));
    TEST_ASSERT_EQUAL_STRING(qiniu_ng_str_map_get(map, QINIU_NG_CHARS("pandora")), QINIU_NG_CHARS("潘多拉"));

    int score = 0;
    qiniu_ng_str_map_each_entry(map, test_qiniu_ng_str_map_handler, &score);
    TEST_ASSERT_EQUAL_INT(score, 10);
    qiniu_ng_str_map_free(&map);
}

bool test_qiniu_ng_str_map_handler(const qiniu_ng_char_t *key, const qiniu_ng_char_t *value, void *score) {
    if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("qiniu")) == 0) {
        TEST_ASSERT_EQUAL_STRING(value, QINIU_NG_CHARS("七牛"));
        (*(int *) score) += 1;
        return true;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("kodo")) == 0) {
        TEST_ASSERT_EQUAL_STRING(value, QINIU_NG_CHARS("科多兽"));
        (*(int *) score) += 2;
        return true;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("dora")) == 0) {
        TEST_ASSERT_EQUAL_STRING(value, QINIU_NG_CHARS("多啦A梦"));
        (*(int *) score) += 3;
        return true;
    } else if (QINIU_NG_CHARS_CMP(key, QINIU_NG_CHARS("pandora")) == 0) {
        TEST_ASSERT_EQUAL_STRING(value, QINIU_NG_CHARS("潘多拉"));
        (*(int *) score) += 4;
        return true;
    } else {
        return false;
    }
}
