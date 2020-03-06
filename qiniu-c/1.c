#include "libqiniu_ng.h"
#include <string.h>

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *url = "http://api.example.com/qiniu/upload/callback";
    const char *content_type = "application/json";
    const char *authorization = "QBox [Qiniu Access Key]:[Authorization Token]";
    const char *body = "{\"key\":\"github-x.png\",\"hash\":\"FqKXVdTvIx_mPjOYdjDyUSy_H1jr\",\"fsize\":6091,\"bucket\":\"if-pbl\",\"name\":\"github logo\"}";
    qiniu_ng_credential_t credential = qiniu_ng_credential_new(access_key, secret_key);
    _Bool is_valid = qiniu_ng_credential_validate_qiniu_callback_request(credential, url, authorization, content_type, body, strlen(body));

    qiniu_ng_credential_free(&credential);
    return 0;
}
