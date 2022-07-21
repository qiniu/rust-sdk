# Qiniu-Upload-Token

[![qiniu-upload-token](https://img.shields.io/crates/v/qiniu-upload-token.svg)](https://crates.io/crates/qiniu-upload-token)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-upload-token)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

负责配置七牛对象上传所需要的上传策略，并提供生成上传凭证的库函数，同时提供 `UploadTokenProvider` 方便扩展获取上传凭证的方式。
同时提供阻塞接口和异步接口（异步接口需要启用 `async` 功能）。
提供 `UploadTokenProvider` 的多个实现方式，例如：

- `StaticUploadTokenProvider` 根据其他服务计算得到的上传凭证字符串生成上传凭证
- `FromUploadPolicy` 根据给出的上传策略和认证信息生成上传凭证
- `BucketUploadTokenProvider` 基于存储空间和认证信息即时生成上传凭证
- `ObjectUploadTokenProvider` 基于存储空间，对象名称和认证信息即时生成上传凭证
- `CachedUploadTokenProvider` 缓存生成的上传凭证，不必每次都即时生成

## 安装

### 不启用异步接口

```toml
[dependencies]
qiniu-upload-token = "0.1.3"
```

### 启用异步接口

```toml
[dependencies]
qiniu-upload-token = { version = "0.1.3", features = ["async"] }
```

## 代码示例

### 阻塞代码示例

### 创建上传策略，并基于该策略创建凭证

```rust
use qiniu_upload_token::{FileType, UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let upload_policy = UploadPolicy::new_for_object("your-bucket", "your-key", Duration::from_secs(3600))
    .file_type(FileType::InfrequentAccess)
    .build();
let credential = Credential::new("your-access-key", "your-secret-key");
let upload_token = upload_policy
    .into_dynamic_upload_token_provider(credential)
    .to_token_string(Default::default())?;
```

### 从其他应用程序生成的上传凭证解析出上传策略

```rust
use qiniu_upload_token::{StaticUploadTokenProvider, prelude::*};

let upload_token: StaticUploadTokenProvider = "your-access-key:qRD-BSf_XGtovGsuOePTc1EKJo8=:eyJkZWFkbGluZSI6MTY0NzgyODY3NCwic2NvcGUiOiJ5b3VyLWJ1Y2tldC1uYW1lIn0=".parse()?;
let access_key = upload_token.access_key(Default::default())?;
let bucket_name = upload_token.bucket_name(Default::default())?;
let upload_policy = upload_token.policy(Default::default())?;
```

### 异步代码示例

### 创建上传策略，并基于该策略创建凭证

```rust
use qiniu_upload_token::{FileType, UploadPolicy, credential::Credential, prelude::*};
use std::time::Duration;

let upload_policy = UploadPolicy::new_for_object("your-bucket", "your-key", Duration::from_secs(3600))
    .file_type(FileType::InfrequentAccess)
    .build();
let credential = Credential::new("your-access-key", "your-secret-key");
let upload_token = upload_policy
    .into_dynamic_upload_token_provider(credential)
    .async_to_token_string(Default::default()).await?;
```

### 从其他应用程序生成的上传凭证解析出上传策略

```rust
use qiniu_upload_token::{StaticUploadTokenProvider, prelude::*};

let upload_token: StaticUploadTokenProvider = "your-access-key:qRD-BSf_XGtovGsuOePTc1EKJo8=:eyJkZWFkbGluZSI6MTY0NzgyODY3NCwic2NvcGUiOiJ5b3VyLWJ1Y2tldC1uYW1lIn0=".parse()?;
let access_key = upload_token.async_access_key(Default::default()).await?;
let bucket_name = upload_token.async_bucket_name(Default::default()).await?;
let upload_policy = upload_token.async_policy(Default::default()).await?;
```

## 最低支持的 Rust 版本（MSRV）

1.60.0

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
