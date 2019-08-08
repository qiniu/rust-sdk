#include "unity.h"
#include "test.h"
#include <stdio.h>

void write_str_to_file(const char* path, const char* content) {
    FILE *fp = fopen(path, "w+");
    if (fp == NULL) {
        TEST_FAIL_MESSAGE("fopen() failed");
    }
    fprintf(fp, "%s", content);
    fclose(fp);
}
