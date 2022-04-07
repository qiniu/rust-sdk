# Qiniu Resource Storage SDK for Rust

[![Run Test Cases](https://github.com/bachue/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/bachue/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/bachue/rust-sdk.svg?label=release)](https://github.com/bachue/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bachue/rust-sdk/blob/master/LICENSE)

## 概要

Qiniu SDK for Rust 包含以下特性：

- 通过提供多个不同的 Crate，为不同层次的开发都提供了方便易用的编程接口。
- 同时提供阻塞 IO 接口和基于 Async/Await 的异步 IO 接口。
- 提供大量的可供二次开发的 Trait，方便灵活定制，例如 HTTP 客户端提供了 `ureq`，`reqwest` 和 `isahc` 三种不同的库实现，也可以基于 `qiniu-http` 自行定制开发接入其他 HTTP 客户端实现；又例如 DNS 客户端提供了 `libc`，`c-ares`，`trust-dns` 三种不同的库实现，也可以基于 [Resolver](https://docs.rs/qiniu-http-client/latest/qiniu_http_client/trait.Resolver.html) 自行定制开发接入其他 DNS 客户端实现。

## 安装

Qiniu SDK for Rust 包含以下 Crates:

| Crate 名称      | 文档 | 描述 |
| ----------- | ----------- | ----------- |
| [![qiniu-etag](https://img.shields.io/crates/v/qiniu-etag.svg)](https://crates.io/crates/qiniu-etag)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-etag) | Etag 算法库，实现七牛 Etag 算法       |
| [![qiniu-credential](https://img.shields.io/crates/v/qiniu-credential.svg)](https://crates.io/crates/qiniu-credential)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-credential) | 七牛认证库，实现七牛认证接口以及签名相关算法       |
| [![qiniu-upload-token](https://img.shields.io/crates/v/qiniu-upload-token.svg)](https://crates.io/crates/qiniu-upload-token)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-upload-token) | 七牛上传凭证，实现七牛上传策略和上传凭证接口以及相关算法      |
| [![qiniu-http](https://img.shields.io/crates/v/qiniu-http.svg)](https://crates.io/crates/qiniu-http)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http) | 七牛客户端 HTTP 接口，为不同的 HTTP 客户端实现提供相同的基础接口      |
| [![qiniu-ureq](https://img.shields.io/crates/v/qiniu-ureq.svg)](https://crates.io/crates/qiniu-ureq)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-ureq) | 基于 [Ureq](https://docs.rs/ureq) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-isahc](https://img.shields.io/crates/v/qiniu-isahc.svg)](https://crates.io/crates/qiniu-isahc)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-isahc) | 基于 [Isahc](https://docs.rs/isahc) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-reqwest](https://img.shields.io/crates/v/qiniu-reqwest.svg)](https://crates.io/crates/qiniu-reqwest)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-reqwest) | 基于 [Reqwest](https://docs.rs/reqwest) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-http-client](https://img.shields.io/crates/v/qiniu-http-client.svg)](https://crates.io/crates/qiniu-http-client)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http-client) | 基于 [qiniu-http](https://docs.rs/qiniu-http) 提供具有重试功能的 HTTP 客户端       |
| [![qiniu-apis](https://img.shields.io/crates/v/qiniu-apis.svg)](https://crates.io/crates/qiniu-apis)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-apis) | 实现七牛 API 调用客户端接口       |
| [![qiniu-objects-manager](https://img.shields.io/crates/v/qiniu-objects-manager.svg)](https://crates.io/crates/qiniu-objects-manager)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-objects-manager) | 实现七牛对象相关管理接口，包含对象的列举和操作       |
| [![qiniu-upload-manager](https://img.shields.io/crates/v/qiniu-upload-manager.svg)](https://crates.io/crates/qiniu-upload-manager)      | [![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-upload-manager) | 实现七牛对象上传功能       |

## 代码示例

### 客户端上传凭证

#### 简单上传的凭证

```rust
use qiniu_upload_token::{UploadPolicy, credential::Credential, prelude::*};
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

```rust
use qiniu_upload_token::{UploadPolicy, credential::Credential, prelude::*};
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

```rust
use qiniu_upload_token::{UploadPolicy, credential::Credential, prelude::*};
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

#### 带回调业务服务器的凭证


```rust
use qiniu_upload_token::{UploadPolicy, credential::Credential, prelude::*};
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

```rust
use qiniu_upload_token::{UploadPolicy, credential::Credential, prelude::*};
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

### 服务端直传

#### 文件上传

```rust
use qiniu_upload_manager::{
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

#### 字节数组上传 / 数据流上传

```rust
use qiniu_upload_manager::{
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
use qiniu_upload_manager::{
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
    .insert_metadata("x:fname", "123.jpg")
    .insert_metadata("x:age", "20")
    .build();
uploader.upload_path("/home/qiniu/test.mp4", params)?;
```

### 下载文件

#### 公开空间

```rust
use http::Uri;

let object_name = "公司/存储/qiniu.jpg";
let domain = "devtools.qiniu.com";
let mut path = "/".to_string();
url_escape::encode_path_to_string(object_name, &mut path);
let url = Uri::builder()
    .scheme("http")
    .authority(domain)
    .path_and_query(path)
    .build()?;
println!("{}", url);
```

#### 私有空间

```rust
use qiniu_credential::Credential;
use http::Uri;

let access_key = "access key";
let secret_key = "secret key";
let object_name = "公司/存储/qiniu.jpg";
let domain = "devtools.qiniu.com";
let mut path = "/".to_string();
url_escape::encode_path_to_string(object_name, &mut path);
let url = Uri::builder()
    .scheme("http")
    .authority(domain)
    .path_and_query(path)
    .build()?;
let credential = Credential::new(access_key, secret_key);
let url = credential.sign_download_url(url, Duration::from_secs(3600));
println!("{}", url);
```

### 资源管理

#### 获取文件信息

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);

bucket
    .modify_object_metadata(object_name, mime::APPLICATION_JSON)
    .call()?;
```

#### 移动或重命名文件

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let to_bucket_name = "to bucket name";
let to_object_name = "new object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);

bucket
    .move_object_to(object_name, to_bucket_name, to_object_name);
    .call()?;
```

#### 复制文件副本

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let to_bucket_name = "to bucket name";
let to_object_name = "new object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);

bucket
    .copy_object_to(object_name, to_bucket_name, to_object_name);
    .call()?;
```

#### 删除空间中的文件

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);

bucket
    .delete_object(object_name);
    .call()?;
```

#### 设置或更新文件的生存时间

```rust
use qiniu_objects_manager::{apis::credential::Credential, AfterDays, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let object_name = "object name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);

bucket
    .modify_object_life_cycle(object_name)
    .delete_after_days(AfterDays::new(10))?;
```

#### 获取空间文件列表

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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

### 资源管理批量操作

#### 批量获取文件信息

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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

#### 批量删除文件类型

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
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
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let access_key = "access key";
let secret_key = "secret key";
let bucket_name = "bucket name";
let credential = Credential::new(access_key, secret_key);
let object_manager = ObjectsManager::builder(credential).build();
let bucket = object_manager.bucket(bucket_name);
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.unfreeze_object("qiniu.jpg", 7));
ops.add_operation(bucket.unfreeze_object("qiniu.png", 7));
ops.add_operation(bucket.unfreeze_object("qiniu.mp4", 7));

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

1.56.0

## 联系我们

- 如果需要帮助，请提交工单（在portal右侧点击咨询和建议提交工单，或者直接向 support@qiniu.com 发送邮件）
- 如果有什么问题，可以到问答社区提问，[问答社区](http://qiniu.segmentfault.com/)
- 更详细的文档，见[官方文档站](http://developer.qiniu.com/)
- 如果发现了bug， 欢迎提交 [Issue](https://github.com/bachue/rust-sdk/issues)
- 如果有功能需求，欢迎提交 [Issue](https://github.com/bachue/rust-sdk/issues)
- 如果要提交代码，欢迎提交 [Pull Request](https://github.com/bachue/rust-sdk/pulls)
- 欢迎关注我们的[微信](https://www.qiniu.com/contact) [微博](http://weibo.com/qiniutek)，及时获取动态信息。

## 代码许可

This project is licensed under the [MIT license].

[MIT license]: https://github.com/bachue/rust-sdk/blob/master/LICENSE
