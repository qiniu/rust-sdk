# 七牛云新一代 C SDK

此 C SDK 基于 [七牛云官方 API](http://developer.qiniu.com/) 构建。
使用此 SDK 构建您的网络应用程序，能让您以非常便捷地方式将数据安全地存储到七牛云上。
无论您的网络应用是一个网站程序，还是包括从云端（服务端程序）到终端（手持设备应用）的架构的服务或应用，通过七牛云及其 SDK，都能让您应用程序的终端用户高速上传和下载，同时也让您的服务端更加轻盈。

C SDK 属于七牛服务端 SDK 之一，主要有如下功能：
1. 提供生成客户端上传所需的上传凭证的功能
2. 提供文件从服务端直接上传七牛的功能
3. 提供文件从七牛直接下载到本地的功能 【开发中】
4. 提供对七牛空间中文件进行管理的功能 【开发中】
5. 提供对七牛空间中文件进行处理的功能 【开发中】
6. 提供七牛 CDN 相关的刷新，预取，日志功能 【开发中】

# 鉴权

七牛 C SDK 的所有的功能，都需要合法的授权。授权凭证的签算需要七牛账号下的一对有效的 `Access Key` 和 `Secret Key` ，这对密钥可以通过如下步骤获得：

1. 点击[注册](https://portal.qiniu.com/signup)开通七牛开发者帐号
2. 如果已有账号，直接登录七牛开发者后台，点击[这里](https://portal.qiniu.com/user/key)查看 Access Key 和 Secret Key

# 存储空间

七牛对象存储的文件需要存储在存储空间中，存储空间即可以通过[七牛开发者后台](https://portal.qiniu.com/kodo/bucket)进行管理，也可以调用 C SDK 提供的 API 进行管理。

## 创建存储空间

在创建存储空间时，需要注意存储空间的名称必须遵守以下规则：
- 存储空间名称不允许重复，遇到冲突请更换名称。
- 名称由 3 ~ 63 个字符组成 ，可包含小写字母、数字和短划线，且必须以小写字母或者数字开头和结尾。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *new_bucket_name = "[New Bucket Name, Must be Unique]";
    qiniu_ng_client_t client = qiniu_ng_client_new_default(access_key, secret_key);
    qiniu_ng_err_t err;

    if (!qiniu_ng_storage_create_bucket(client, new_bucket_name, qiniu_ng_region_z1, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_client_free(&client);
        return 1;
    }

    qiniu_ng_client_free(&client);
    return 0;
}
```

## 删除存储空间

删除存储空间前务必保证存储空间里已经没有任何文件，否则删除将会失败。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "Bucket Name";
    qiniu_ng_client_t client = qiniu_ng_client_new_default(access_key, secret_key);
    qiniu_ng_err_t err;

    if (!qiniu_ng_storage_drop_bucket(client, bucket_name, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_client_free(&client);
        return 1;
    }

    qiniu_ng_client_free(&client);
    return 0;
}
```

## 列出存储空间

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    qiniu_ng_client_t client = qiniu_ng_client_new_default(access_key, secret_key);
    qiniu_ng_str_list_t bucket_names;
    qiniu_ng_err_t err;

    if (!qiniu_ng_storage_bucket_names(client, &bucket_names, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_client_free(&client);
        return 1;
    }

    size_t bucket_names_len = qiniu_ng_str_list_len(bucket_names);
    for (size_t i = 0; i < bucket_names_len; i++) {
        printf("%s\n", qiniu_ng_str_list_get(bucket_names, i));
    }

    qiniu_ng_str_list_free(&bucket_names);
    qiniu_ng_client_free(&client);
    return 0;
}
```

# 上传
## 上传流程

七牛文件上传分为客户端上传（主要是指网页端和移动端等面向终端用户的场景）和服务端上传两种场景，具体可以参考文档[七牛业务流程](https://developer.qiniu.com/kodo/manual/programming-model)。

服务端 SDK 在上传方面主要提供两种功能，一种是生成客户端上传所需要的上传凭证，另外一种是直接上传文件到云端。

## 客户端上传凭证

客户端（移动端或者 Web 端）上传文件的时候，需要从客户自己的业务服务器获取上传凭证，而这些上传凭证是通过服务端的 SDK 来生成的，然后通过客户自己的业务 API 分发给客户端使用。根据上传的业务需求不同，七牛云 C SDK 支持丰富的上传凭证生成方式。

### 简单上传凭证

最简单的上传凭证只需要 `AccessKey`，`SecretKey` 和 `Bucket` 就可以。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket("[Bucket Name]", config);
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

默认情况下，在不指定上传凭证的有效时间情况下，默认有效期为 1 个小时。也可以自行指定上传凭证的有效期，例如：

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(bucket_name, config);
    qiniu_ng_upload_policy_builder_set_token_lifetime(upload_policy_builder, 7200);
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 覆盖上传凭证

覆盖上传除了需要 `简单上传` 所需要的信息之外，还需要想进行覆盖的文件名称，这个文件名称同时可是客户端上传代码中指定的文件名，两者必须一致。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 自定义上传回复凭证

默认情况下，文件上传到七牛之后，在没有设置 `return_body` 或者回调相关的参数情况下，七牛返回给上传端的回复格式为 `hash`和 `key`，例如：

```json
{"hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","key":"qiniu.jpg"}
```

有时候我们希望能自定义这个返回的 JSON 格式的内容，可以通过设置 `return_body` 参数来实现，在 `return_body` 中，我们可以使用七牛支持的[魔法变量](https://developer.qiniu.com/kodo/manual/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/vars#xvar)。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_return_body(upload_policy_builder, "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

则文件上传到七牛之后，收到的回复内容格式如下：

```json
{"key":"github-x.png","hash":"FqKXVdTvIx_mPjOYdjDyUSy_H1jr","fsize":6091,"bucket":"if-pbl","name":"github logo"}
```

对于上面的自定义返回值，我们可以调用 `qiniu_ng_upload_response_get_string(upload_response)` 的方法输出这个结果：

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    const char *file_path = "/local/file/path";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(upload_manager, access_key, secret_key, 0);

    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_return_body(upload_policy_builder, "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_response_t upload_response;
    qiniu_ng_upload_params_t params = {
        .key = "qiniu.mp4",
        .file_name = "local file name",
    };
    qiniu_ng_err_t err;
    if (!qiniu_ng_bucket_uploader_upload_file_path(bucket_uploader, upload_token, file_path, &params, &upload_response, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_upload_token_free(&upload_token);
        qiniu_ng_bucket_uploader_free(&bucket_uploader);
        qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
        qiniu_ng_upload_manager_free(&upload_manager);
        qiniu_ng_config_free(&config);
        return 1;
    }

    qiniu_ng_str_t upload_response_string = qiniu_ng_upload_response_get_string(upload_response);
    printf("%s\n", qiniu_ng_str_get_ptr(upload_response_string));
    qiniu_ng_str_free(&upload_response_string);
    qiniu_ng_upload_response_free(&upload_response);
    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 带回调业务服务器的凭证

上面生成的 `自定义上传回复` 的上传凭证适用于上传端（无论是客户端还是服务端）和七牛服务器之间进行直接交互的情况下。在客户端上传的场景之下，有时候客户端需要在文件上传到七牛之后，从业务服务器获取相关的信息，这个时候就要用到七牛的上传回调及相关回调参数的设置。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    const char *callback_url = "http://api.example.com/qiniu/upload/callback";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_callback(upload_policy_builder, &callback_url, 1, NULL, "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}", "application/json");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

在使用了上传回调的情况下，客户端收到的回复就是业务服务器响应七牛的 JSON 格式内容，业务服务器收到回调之后必须响应 JSON 格式的回复給七牛，这个回复会被七牛传递给客户端。

例如上面的 `callback` 的设置会在文件上传到七牛之后，触发七牛回调如下内容給业务服务器：

```json
{"key":"github-x.png","hash":"FqKXVdTvIx_mPjOYdjDyUSy_H1jr","fsize":6091,"bucket":"if-pbl","name":"github logo"}
```

通常情况下，我们建议使用 `application/json` 格式来设置 `callback` ，保持数据格式的统一性。实际情况下， `callback` 也支持 `application/x-www-form-urlencoded` 格式来组织内容，这个主要看业务服务器在接收到 `callback` 的内容时如何解析。例如：

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    const char *callback_url = "http://api.example.com/qiniu/upload/callback";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_callback(upload_policy_builder, &callback_url, 1, NULL, "key=$(key)&hash=$(etag)&bucket=$(bucket)&fsize=$(fsize)&name=$(x:name)", "application/x-www-form-urlencoded");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 带自定义参数的凭证

七牛支持客户端上传文件的时候定义一些自定义参数，这些参数可以在 `return_body` 和 `callback` 里面和七牛内置支持的魔法变量（即系统变量）通过相同的方式来引用。这些自定义的参数名称必须以 `x:` 开头。例如客户端上传的时候指定了自定义的参数 `x:name` 和 `x:age` 分别是 string 和 int 类型。那么可以通过下面的方式引用：

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_return_body(upload_policy_builder, "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\",\"age\":$(x:age)}");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

或者

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *key_to_overwrite = "qiniu.mp4";
    const char *callback_url = "http://api.example.com/qiniu/upload/callback";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_object(bucket_name, key_to_overwrite, config);
    qiniu_ng_upload_policy_builder_set_callback(upload_policy_builder, &callback_url, 1, NULL, "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\",\"age\":$(x:age)}", "application/json");
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 综合上传凭证

上面的生成上传凭证的方法，都是通过设置[上传策略](https://developer.qiniu.com/kodo/manual/put-policy)相关的参数来支持的，这些参数可以通过不同的组合方式来满足不同的业务需求，可以灵活地组织你所需要的上传凭证。

## 服务器直传

服务端直传是指客户利用七牛服务端 SDK 从服务端直接上传文件到七牛云，交互的双方一般都在机房里面，所以服务端可以自己生成上传凭证，然后利用 SDK 中的上传逻辑进行上传，最后从七牛云获取上传的结果，这个过程中由于双方都是业务服务器，所以很少利用到上传回调的功能，而是直接自定义 `return_body` 来获取自定义的回复内容。

### 文件上传

最简单的就是上传本地文件，直接指定文件的完整路径即可上传。

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    const char *file_path = "/local/file/path";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(upload_manager, access_key, secret_key, 0);

    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(bucket_name, config);
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_err_t err;
    if (!qiniu_ng_bucket_uploader_upload_file_path(bucket_uploader, upload_token, file_path, NULL, NULL, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_upload_token_free(&upload_token);
        qiniu_ng_bucket_uploader_free(&bucket_uploader);
        qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
        qiniu_ng_upload_manager_free(&upload_manager);
        qiniu_ng_config_free(&config);
        return 1;
    }

    qiniu_ng_str_free(&upload_response_string);
    qiniu_ng_upload_response_free(&upload_response);
    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 输入流上传

如果要上传的数据存在于输入流中，可以使用基于 `FILE` 的流上传数据。这里给出一个将 STDIN 输入流上传的例子：

```c
#include "libqiniu_ng.h"

int main() {
    const char *access_key = "[Qiniu Access Key]";
    const char *secret_key = "[Qiniu Secret Key]";
    const char *bucket_name = "[Bucket Name]";
    qiniu_ng_config_t config = qiniu_ng_config_new_default();
    qiniu_ng_upload_manager_t upload_manager = qiniu_ng_upload_manager_new(config);
    qiniu_ng_bucket_uploader_t bucket_uploader = qiniu_ng_bucket_uploader_new_from_bucket_name(upload_manager, access_key, secret_key, 0);

    qiniu_ng_upload_policy_builder_t upload_policy_builder = qiniu_ng_upload_policy_builder_new_for_bucket(bucket_name, config);
    qiniu_ng_upload_token_t upload_token = qiniu_ng_upload_token_new_from_policy_builder(upload_policy_builder, access_key, secret_key);

    qiniu_ng_err_t err;
    if (!qiniu_ng_bucket_uploader_upload_file(bucket_uploader, upload_token, stdin, NULL, NULL, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_upload_token_free(&upload_token);
        qiniu_ng_bucket_uploader_free(&bucket_uploader);
        qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
        qiniu_ng_upload_manager_free(&upload_manager);
        qiniu_ng_config_free(&config);
        return 1;
    }

    qiniu_ng_str_free(&upload_response_string);
    qiniu_ng_upload_response_free(&upload_response);
    qiniu_ng_upload_token_free(&upload_token);
    qiniu_ng_bucket_uploader_free(&bucket_uploader);
    qiniu_ng_upload_policy_builder_free(&upload_policy_builder);
    qiniu_ng_upload_manager_free(&upload_manager);
    qiniu_ng_config_free(&config);
    return 0;
}
```

### 自定义数据上传

对于数据存在于内存中，无法使用文件路径或输入流的情况，可以考虑定义基于 `qiniu_ng_readable_t` 的回调函数，并且调用 `qiniu_ng_bucket_uploader_upload_reader()` 函数上传。

### 文件上传策略

默认情况下，对于尺寸大于 4 MB 的文件，SDK 默认自动使用分片上传的方式来上传，分片上传通过将一个文件切割为标准的块（默认的固定大小为 4 MB，可以通过修改配置增加尺寸，但必须是 4 MB 的倍数），然后通过上传块的方式来进行文件的上传。一个块中的片和另外一个块中的片是可以并发的。分片上传不等于断点续传，但是分片上传可以支持断点续传。

断点续传是将每个块上传完毕的返回的 `context` 保存到本地的文件中持久化，如果本次上传被中断，下次可以从这个进度文件中读取每个块上传的状态，然后继续上传完毕没有完成的块，最后完成文件的拼接。

这里需要注意，只有在块上传完毕之后，才向本地的进度文件写入 `context` 内容。

另外需要注意，每个 `context` 的有效期最长默认是 `7` 天，过期的 `context` 会触发 `701` 的错误，默认情况下，如果 `context` 超过 7 天，SDK 会自动重新上传 `context` 对应的分块。

上述策略中不少参数可以在 `qiniu_ng_config_builder_t` 中配置，这里给出一个修改配置参数的例子：

```c
#include "libqiniu_ng.h"

int main() {
    qiniu_ng_config_builder_t config_builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_upload_threshold(config_builder, 16 * 1024 * 1024); // 修改成尺寸大于 16 MB 的文件才使用分片上传
    qiniu_ng_config_builder_upload_block_size(config_builder, 8 * 1024 * 1024); // 每个分块尺寸修改为 8 MB
    qiniu_ng_config_builder_upload_recorder_root_directory(config_builder, "/recorder/data"); // 修改上传进度记录文件的存储目录
    qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(config_builder,5 * 24 * 60 * 60); // 每个分块的有效期减少为 5 天

    qiniu_ng_config_t config;
    qiniu_ng_err_t err;
    if(!qiniu_ng_config_build(&config_builder, &config, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_config_builder_free(&config_builder);
        return 1;
    }
    qiniu_ng_config_builder_free(&config_builder);

    // 继续使用 config ...

    return 0;
}
```

## 私有云配置

默认情况下，C SDK 内置了七牛公有云存储的配置。如果需要使用七牛私有云，则需要对 `qiniu_ng_config_builder_t` 中的配置作出必要的调整，这里给出一个例子：

```c
#include "libqiniu_ng.h"

int main() {
    qiniu_ng_config_builder_t config_builder = qiniu_ng_config_builder_new();
    qiniu_ng_config_builder_use_https(config_builder, true); // 设置为使用 HTTPS 协议
    qiniu_ng_config_builder_uc_host(config_builder, "uc.example.com"); // 设置 UC 服务器地址
    qiniu_ng_config_builder_rs_host(config_builder, "rs.example.com"); // 设置 RS 服务器地址
    qiniu_ng_config_builder_rsf_host(config_builder, "rsf.example.com"); // 设置 RSF 服务器地址
    qiniu_ng_config_builder_api_host(config_builder, "api.example.com");  // 设置 API 服务器地址

    qiniu_ng_config_t config;
    qiniu_ng_err_t err;
    if(!qiniu_ng_config_build(&config_builder, &config, &err)) {
        qiniu_ng_err_fprintf(stderr, "%s\n", err);
        qiniu_ng_err_ignore(&err);
        qiniu_ng_config_builder_free(&config_builder);
        return 1;
    }
    qiniu_ng_config_builder_free(&config_builder);

    // 继续使用 config ...

    return 0;
}
```
