# Qiniu Resource Storage SDK for Rust

[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概要

Qiniu SDK for Rust 包含以下特性：

- 通过提供多个不同的 Crate，为不同层次的开发都提供了方便易用的编程接口。
- 同时提供阻塞 IO 接口和基于 Async/Await 的异步 IO 接口。
- 提供大量的可供二次开发的 Trait，方便灵活定制，例如 HTTP 客户端提供了 `ureq`，`reqwest` 和 `isahc` 三种不同的库实现，也可以基于 `qiniu-http` 自行定制开发接入其他 HTTP 客户端实现；又例如 DNS 客户端提供了 `libc`，`c-ares`，`trust-dns` 三种不同的库实现，也可以基于 [Resolver](https://docs.rs/qiniu-http-client/latest/qiniu_http_client/trait.Resolver.html) 自行定制开发接入其他 DNS 客户端实现。

## 安装

Qiniu SDK for Rust 包含以下 Crates:

| Crate 链接                                                                                                                               | 文档                                                                                                    | 描述                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [![qiniu-etag](https://img.shields.io/crates/v/qiniu-etag.svg)](https://crates.io/crates/qiniu-etag)                                     | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-etag)             | Etag 算法库，实现七牛 Etag 算法                                                                                                                                                                                                                                                    |
| [![qiniu-credential](https://img.shields.io/crates/v/qiniu-credential.svg)](https://crates.io/crates/qiniu-credential)                   | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-credential)       | 七牛认证库，实现七牛认证接口以及签名相关算法                                                                                                                                                                                                                                       |
| [![qiniu-upload-token](https://img.shields.io/crates/v/qiniu-upload-token.svg)](https://crates.io/crates/qiniu-upload-token)             | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-upload-token)     | 七牛上传凭证，实现七牛上传策略和上传凭证接口以及相关算法                                                                                                                                                                                                                           |
| [![qiniu-http](https://img.shields.io/crates/v/qiniu-http.svg)](https://crates.io/crates/qiniu-http)                                     | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http)             | 七牛客户端 HTTP 接口，为不同的 HTTP 客户端实现提供相同的基础接口                                                                                                                                                                                                                   |
| [![qiniu-ureq](https://img.shields.io/crates/v/qiniu-ureq.svg)](https://crates.io/crates/qiniu-ureq)                                     | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-ureq)             | 基于 [Ureq](https://docs.rs/ureq) 库实现七牛客户端 HTTP 接口，对于使用 `qiniu-http-request`，`qiniu-apis`，`qiniu-objects-manager` 或 `qiniu-upload-manager` 的用户，可以直接启用 `ureq` 功能，将默认使用该 HTTP 客户端实现。需要注意的是，如果需要使用异步接口，则不能选择 `ureq` |
| [![qiniu-isahc](https://img.shields.io/crates/v/qiniu-isahc.svg)](https://crates.io/crates/qiniu-isahc)                                  | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-isahc)            | 基于 [Isahc](https://docs.rs/isahc) 库实现七牛客户端 HTTP 接口，对于使用 `qiniu-http-request`，`qiniu-apis`，`qiniu-objects-manager` 或 `qiniu-upload-manager` 的用户，可以直接启用 `isahc` 功能，将默认使用该 HTTP 客户端实现                                                     |
| [![qiniu-reqwest](https://img.shields.io/crates/v/qiniu-reqwest.svg)](https://crates.io/crates/qiniu-reqwest)                            | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-reqwest)          | 基于 [Reqwest](https://docs.rs/reqwest) 库实现七牛客户端 HTTP 接口，对于使用 `qiniu-http-request`，`qiniu-apis`，`qiniu-objects-manager` 或 `qiniu-upload-manager` 的用户，可以直接启用 `reqwest` 功能，将默认使用该 HTTP 客户端实现                                               |
| [![qiniu-http-client](https://img.shields.io/crates/v/qiniu-http-client.svg)](https://crates.io/crates/qiniu-http-client)                | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http-client)      | 基于 [qiniu-http](https://docs.rs/qiniu-http) 提供具有重试功能的 HTTP 客户端                                                                                                                                                                                                       |
| [![qiniu-apis](https://img.shields.io/crates/v/qiniu-apis.svg)](https://crates.io/crates/qiniu-apis)                                     | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-apis)             | 实现七牛 API 调用客户端接口                                                                                                                                                                                                                                                        |
| [![qiniu-objects-manager](https://img.shields.io/crates/v/qiniu-objects-manager.svg)](https://crates.io/crates/qiniu-objects-manager)    | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-objects-manager)  | 实现七牛对象相关管理接口，包含对象的列举和操作                                                                                                                                                                                                                                     |
| [![qiniu-upload-manager](https://img.shields.io/crates/v/qiniu-upload-manager.svg)](https://crates.io/crates/qiniu-upload-manager)       | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-upload-manager)   | 实现七牛对象上传功能                                                                                                                                                                                                                                                               |
| [![qiniu-download-manager](https://img.shields.io/crates/v/qiniu-download-manager.svg)](https://crates.io/crates/qiniu-download-manager) | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-download-manager) | 实现七牛对象下载功能                                                                                                                                                                                                                                                               |
| [![qiniu-sdk](https://img.shields.io/crates/v/qiniu-sdk.svg)](https://crates.io/crates/qiniu-sdk)                                        | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-sdk)              | 七牛 SDK 入口                                                                                                                                                                                                                                                                      |

## 代码示例

以下所有代码示例都以 `qiniu-sdk` 作为入口。

### 客户端上传凭证（需要依赖 `qiniu-sdk` 启用 `upload-token` 功能）

客户端（移动端或者Web端）上传文件的时候，需要从客户自己的业务服务器获取上传凭证，而这些上传凭证是通过服务端的 SDK 来生成的，然后通过客户自己的业务API分发给客户端使用。根据上传的业务需求不同，七牛云 Rust SDK 支持丰富的上传凭证生成方式。

#### 简单上传的凭证

最简单的上传凭证只需要 `access key`，`secret key` 和 `bucket` 就可以。

```rust
use qiniu_sdk::upload_token::{UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let upload_token = UploadPolicy::new_for_bucket(bucket_name, Duration::from_secs(3600))
    .build_token(credential, Default::default())?;
println!("{}", upload_token);
```

#### 覆盖上传的凭证

覆盖上传除了需要简单上传所需要的信息之外，还需要想进行覆盖的对象名称 `object name`，这个对象名称同时是客户端上传代码中指定的对象名称，两者必须一致。

```rust
use qiniu_sdk::upload_token::{UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_token = UploadPolicy::new_for_object(bucket_name, object_name, Duration::from_secs(3600))
    .build_token(credential, Default::default())?;
println!("{}", upload_token);
```

#### 自定义上传回复的凭证

默认情况下，文件上传到七牛之后，在没有设置 `return_body` 或者回调相关的参数情况下，七牛返回给上传端的回复格式为 `hash` 和 `key`，例如：

```json
{"hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","key":"qiniu.jpg"}
```

有时候我们希望能自定义这个返回的 JSON 格式的内容，可以通过设置 `return_body` 参数来实现，在 `return_body` 中，我们可以使用七牛支持的魔法变量和自定义变量。

```rust
use qiniu_sdk::upload_token::{UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_token = UploadPolicy::new_for_object(bucket_name, object_name, Duration::from_secs(3600))
    .return_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"bucket\":\"$(bucket)\",\"fsize\":$(fsize)}")
    .build_token(credential, Default::default())?;
println!("{}", upload_token);
```

则文件上传到七牛之后，收到的回复内容如下：

```json
{"key":"qiniu.jpg","hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","bucket":"if-bc","fsize":39335}
```

#### 带回调业务服务器的凭证

上面生成的自定义上传回复的上传凭证适用于上传端（无论是客户端还是服务端）和七牛服务器之间进行直接交互的情况下。在客户端上传的场景之下，有时候客户端需要在文件上传到七牛之后，从业务服务器获取相关的信息，这个时候就要用到七牛的上传回调及相关回调参数的设置。

```rust
use qiniu_sdk::upload_token::{UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_token = UploadPolicy::new_for_object(bucket_name, object_name, Duration::from_secs(3600))
    .callback(&["http://api.example.com/qiniu/upload/callback"], "", "{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"bucket\":\"$(bucket)\",\"fsize\":$(fsize)}", "application/json")
    .build_token(credential, Default::default())?;
println!("{}", upload_token);
```

在使用了上传回调的情况下，客户端收到的回复就是业务服务器响应七牛的JSON格式内容。
通常情况下，我们建议使用 `application/json` 格式来设置 `callback_body`，保持数据格式的统一性。实际情况下，`callback_body` 也支持 `application/x-www-form-urlencoded` 格式来组织内容，这个主要看业务服务器在接收到 `callback_body` 的内容时如何解析。例如：

```rust
use qiniu_sdk::upload_token::{UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_token = UploadPolicy::new_for_object(bucket_name, object_name, Duration::from_secs(3600))
    .callback(&["http://api.example.com/qiniu/upload/callback"], "", "key=$(key)&hash=$(etag)&bucket=$(bucket)&fsize=$(fsize)", "")
    .build_token(credential, Default::default())?;
println!("{}", upload_token);
```

### 服务端直传（需要依赖 `qiniu-sdk` 启用 `upload` 功能）

服务端直传是指客户利用七牛服务端 SDK 从服务端直接上传文件到七牛云，交互的双方一般都在机房里面，所以服务端可以自己生成上传凭证，然后利用 SDK 中的上传逻辑进行上传，最后从七牛云获取上传的结果，这个过程中由于双方都是业务服务器，所以很少利用到上传回调的功能，而是直接自定义 `return_body` 来获取自定义的回复内容。

#### 文件上传

最简单的就是上传本地文件，直接指定文件的完整路径即可上传。

```rust
use qiniu_sdk::upload::{
    apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager,
    UploadTokenSigner,
};
use std::{time::Duration};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
    credential,
    bucket_name,
    Duration::from_secs(3600),
))
.build();
let mut uploader: AutoUploader = upload_manager.auto_uploader();

let params = AutoUploaderObjectParams::builder().object_name(object_name).file_name(object_name).build();
uploader.upload_path("/home/qiniu/test.png", params)?;
```

在这个场景下，`AutoUploader` 会自动根据文件尺寸判定是否启用断点续上传，如果文件较大，上传了一部分时因各种原因从而中断，再重新执行相同的代码时，SDK 会尝试找到先前没有完成的上传任务，从而继续进行上传。

#### 字节数组上传 / 数据流上传

可以支持将内存中的字节数组或实现了 `std::io::Read` 的实例上传到空间中。

```rust
use qiniu_sdk::upload::{
    apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager,
    UploadTokenSigner,
};
use std::{io::Cursor, time::Duration};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
    credential,
    bucket_name,
    Duration::from_secs(3600),
))
.build();
let mut uploader: AutoUploader = upload_manager.auto_uploader();

let params = AutoUploaderObjectParams::builder().object_name(object_name).file_name(object_name).build();
uploader.upload_reader(Cursor::new("hello qiniu cloud"), params)?;
```

#### 自定义参数上传

```rust
use qiniu_sdk::upload::{
    apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager, UploadTokenSigner,
};
use std::{io::Cursor, time::Duration};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_manager = UploadManager::builder(
    UploadTokenSigner::new_credential_provider_builder(credential, bucket_name, Duration::from_secs(3600))
        .on_policy_generated(|builder| {
            builder
                .return_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fname\":\"$(x:fname)\",\"age\",$(x:age)}");
        })
        .build(),
)
.build();
let mut uploader: AutoUploader = upload_manager.auto_uploader();

let params = AutoUploaderObjectParams::builder()
    .object_name(object_name)
    .file_name(object_name)
    .insert_custom_var("fname", "123.jpg")
    .insert_custom_var("age", "20")
    .build();
uploader.upload_path("/home/qiniu/test.mp4", params)?;
```

#### 私有云上传

```rust
use qiniu_sdk::upload::{
    apis::{
        credential::Credential,
        http_client::{EndpointsBuilder, HttpClient},
    },
    AutoUploader, AutoUploaderObjectParams, UploadManager, UploadTokenSigner,
};
use std::{time::Duration};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
    credential,
    bucket_name,
    Duration::from_secs(3600),
))
.uc_endpoints(
    EndpointsBuilder::default()
        .add_preferred_endpoint("ucpub-qos.pocdemo.qiniu.io".into()) // 私有云存储空间管理服务域名，可以添加多个
        .build(),
)
.use_https(false) // 私有云普遍使用 HTTP 协议，而 SDK 则默认为 HTTPS 协议
.build();
let mut uploader: AutoUploader = upload_manager.auto_uploader();

let params = AutoUploaderObjectParams::builder().object_name(object_name).file_name(object_name).build();
uploader.upload_path("/home/qiniu/test.png", params)?;
```

### 下载文件（需要依赖 `qiniu-sdk` 启用 `download` 功能）

文件下载分为公开空间的文件下载和私有空间的文件下载。

#### 公开空间

```rust
use qiniu_sdk::download::{DownloadManager, StaticDomainsUrlsGenerator};

let object_name = "公司/存储/qiniu.jpg";
let domain = "devtools.qiniu.com";
let path = "/home/user/qiniu.jpg";
let download_manager = DownloadManager::new(
    StaticDomainsUrlsGenerator::builder(domain)
        .use_https(false) // 设置为 HTTP 协议
        .build(),
);
download_manager
    .download(object_name)?
    .to_path(path)?;
```

#### 私有空间

```rust
use qiniu_sdk::download::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};

let access_key = "access key";
let secret_key = "secret key";
let object_name = "公司/存储/qiniu.jpg";
let domain = "devtools.qiniu.com";
let path = "/home/user/qiniu.jpg";
let download_manager = DownloadManager::new(UrlsSigner::new(
    Credential::new(access_key, secret_key),
    StaticDomainsUrlsGenerator::builder(domain)
        .use_https(false) // 设置为 HTTP 协议
        .build(),
));
download_manager
    .download(object_name)?
    .to_path(path)?;
```

### 资源管理（需要依赖 `qiniu-sdk` 启用 `objects` 功能）

#### 获取文件信息

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

let response = bucket.stat_object(object_name).call()?;
let entry = response.into_body();
println!("{}", entry.get_hash_as_str());
println!("{}", entry.get_size_as_u64());
println!("{}", entry.get_mime_type_as_str());
println!("{}", entry.get_put_time_as_u64());
```

#### 修改文件类型

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

bucket
    .modify_object_metadata(object_name, mime::APPLICATION_JSON)
    .call()?;
```

#### 移动或重命名文件

移动操作本身支持移动文件到相同，不同空间中，在移动的同时也可以支持文件重命名。唯一的限制条件是，移动的源空间和目标空间必须在同一个机房。

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let to_bucket_name = "to bucket name";
let to_object_name = "new object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

bucket
    .move_object_to(object_name, to_bucket_name, to_object_name);
    .call()?;
```

#### 复制文件副本

文件的复制和文件移动其实操作一样，主要的区别是移动后源文件不存在了，而复制的结果是源文件还存在，只是多了一个新的文件副本。

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let to_bucket_name = "to bucket name";
let to_object_name = "new object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

bucket
    .copy_object_to(object_name, to_bucket_name, to_object_name);
    .call()?;
```

#### 删除空间中的文件

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

bucket
    .delete_object(object_name);
    .call()?;
```

#### 设置或更新文件的生存时间

可以给已经存在于空间中的文件设置文件生存时间，或者更新已设置了生存时间但尚未被删除的文件的新的生存时间。

```rust
use qiniu_sdk::objects::{apis::credential::Credential, AfterDays, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

bucket
    .modify_object_life_cycle(object_name)
    .delete_after_days(AfterDays::new(10))
    .call()?;
```

#### 获取空间文件列表

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);

let mut iter = bucket.list().iter();
while let Some(entry) = iter.next() {
    let entry = entry?;
    println!(
        "{}\n  hash: {}\n  size: {}\n  mime type: {}",
        entry.get_key_as_str(),
        entry.get_hash_as_str(),
        entry.get_size_as_u64(),
        entry.get_mime_type_as_str(),
    );
}
```

#### 私有云中获取空间文件列表

```rust
use qiniu_sdk::objects::{
    apis::{
        credential::Credential,
        http_client::{EndpointsBuilder, HttpClient},
    },
    ObjectsManager,
};
use std::net::Ipv4Addr;

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential)
    .uc_endpoints(
        EndpointsBuilder::default()
            .add_preferred_endpoint("ucpub-qos.pocdemo.qiniu.io".into()) // 私有云存储空间管理服务域名，可以添加多个
            .build(),
    )
    .use_https(false) // 私有云普遍使用 HTTP 协议，而 SDK 则默认为 HTTPS 协议
    .build();
let bucket = object_manager.bucket(bucket_name);

let mut iter = bucket.list().iter();
while let Some(entry) = iter.next() {
    let entry = entry?;
    println!(
        "{}\n  hash: {}\n  size: {}\n  mime type: {}",
        entry.get_key_as_str(),
        entry.get_hash_as_str(),
        entry.get_size_as_u64(),
        entry.get_mime_type_as_str(),
    );
}
```

### 资源管理批量操作（需要依赖 `qiniu-sdk` 启用 `objects` 功能）

#### 批量获取文件信息

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.stat_object("qiniu.jpg"));
ops.add_operation(bucket.stat_object("qiniu.mp4"));
ops.add_operation(bucket.stat_object("qiniu.png"));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(entry) => {
            println!(
                "hash: {:?}\n  size: {:?}\n  mime type: {:?}",
                entry.get_hash_as_str(),
                entry.get_size_as_u64(),
                entry.get_mime_type_as_str(),
            );
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

#### 批量修改文件类型

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.modify_object_metadata("qiniu.jpg", mime::IMAGE_JPEG));
ops.add_operation(bucket.modify_object_metadata("qiniu.png", mime::IMAGE_PNG));
ops.add_operation(bucket.modify_object_metadata("qiniu.mp4", "video/mp4".parse()?));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(_) => {
            println!("Ok");
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

#### 批量删除文件

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.delete_object("qiniu.jpg"));
ops.add_operation(bucket.delete_object("qiniu.png"));
ops.add_operation(bucket.delete_object("qiniu.mp4"));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(_) => {
            println!("Ok");
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

#### 批量移动或重命名文件

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.move_object_to("qiniu.jpg", bucket_name, "qiniu.jpg.move"));
ops.add_operation(bucket.move_object_to("qiniu.png", bucket_name, "qiniu.png.move"));
ops.add_operation(bucket.move_object_to("qiniu.mp4", bucket_name, "qiniu.mp4.move"));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(_) => {
            println!("Ok");
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

#### 批量复制文件

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.copy_object_to("qiniu.jpg", bucket_name, "qiniu.jpg.move"));
ops.add_operation(bucket.copy_object_to("qiniu.png", bucket_name, "qiniu.png.move"));
ops.add_operation(bucket.copy_object_to("qiniu.mp4", bucket_name, "qiniu.mp4.move"));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(_) => {
            println!("Ok");
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

#### 批量解冻归档存储类型文件

```rust
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.restore_archived_object("qiniu.jpg", 7));
ops.add_operation(bucket.restore_archived_object("qiniu.png", 7));
ops.add_operation(bucket.restore_archived_object("qiniu.mp4", 7));

let mut iter = ops.call();
while let Some(result) = iter.next() {
    match result {
        Ok(_) => {
            println!("Ok");
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}
```

## 最低支持的 Rust 版本（MSRV）

- 1.60.0

## 编码规范

- 通过 `cargo clippy` 检查，并经过 `rustfmt` 格式化。
- 所有阻塞操作都提供异步无阻塞版本。
- 竭力避免 unsafe 代码。
- 所有公开的 `trait` 都可以通过 `prelude` 模块自动导入。
- 公开接口中如果要求传入或返回某个第三方库的类型，则该类型也必须公开导出。

## 联系我们

- 如果需要帮助，请提交工单（在portal右侧点击咨询和建议提交工单，或者直接向 support@qiniu.com 发送邮件）
- 如果有什么问题，可以到问答社区提问，[问答社区](http://qiniu.segmentfault.com/)
- 更详细的文档，见[官方文档站](http://developer.qiniu.com/)
- 如果发现了bug， 欢迎提交 [Issue](https://github.com/qiniu/rust-sdk/issues)
- 如果有功能需求，欢迎提交 [Issue](https://github.com/qiniu/rust-sdk/issues)
- 如果要提交代码，欢迎提交 [Pull Request](https://github.com/qiniu/rust-sdk/pulls)
- 欢迎关注我们的[微信](https://www.qiniu.com/contact) [微博](http://weibo.com/qiniutek)，及时获取动态信息。

## 代码许可

This project is licensed under the [MIT license].

[MIT license]: https://github.com/qiniu/rust-sdk/blob/master/LICENSE
