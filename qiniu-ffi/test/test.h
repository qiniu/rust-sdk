#ifndef __TEST_H
#define __TEST_H

#include "libqiniu_ng.h"

/// Utilties
int env_load(char*, bool);

/// Test Cases
void test_qiniu_ng_etag_v1(void);
void test_qiniu_ng_etag_v2(void);
void test_qiniu_ng_etag_from_file(void);

#endif
