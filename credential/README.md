# Qiniu-Credential

[![qiniu-credential](https://img.shields.io/crates/v/qiniu-credential.svg)](https://crates.io/crates/qiniu-credential)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-credential)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

负责存储调用七牛 API 所必要的认证信息，提供 `CredentialProvider` 方便扩展获取认证信息的方式。
同时提供阻塞接口和异步接口（异步接口需要启用 `async` 功能）。
提供 `CredentialProvider` 的多个实现方式，例如：

- `GlobalCredentialProvider` 使用全局变量配置的认证信息
- `EnvCredentialProvider` 使用环境变量配置的认证信息
- `ChainCredentialsProvider` 配置多个 `CredentialProvider` 形成认证信息串，遍历找寻第一个可用的认证信息

### 不启用异步接口

```toml
[dependencies]
qiniu-credential = "0.1.3"
```

### 启用异步接口

```toml
[dependencies]
qiniu-credential = { version = "0.1.3", features = ["async"] }
```

## 代码示例

### 阻塞代码示例

#### 计算七牛鉴权签名 V1

```rust
use qiniu_credential::{Credential, HeaderValue, prelude::*};
use mime::APPLICATION_WWW_FORM_URLENCODED;
use std::io::Cursor;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let authorization = credential
    .get(Default::default())?
    .authorization_v1_for_request_with_body_reader(
        &"http://upload.qiniup.com/".parse()?,
        Some(&HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref())?),
        &mut Cursor::new(b"name=test&language=go"),
    );
```

#### 计算七牛鉴权签名 V2

```rust
use qiniu_credential::{Credential, Method, HeaderMap, HeaderValue, prelude::*};
use http::header::CONTENT_TYPE;
use mime::APPLICATION_JSON;
use std::io::Cursor;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let mut headers = HeaderMap::new();
headers.insert(CONTENT_TYPE, HeaderValue::from_str(APPLICATION_JSON.as_ref())?);
let authorization = credential
    .get(Default::default())?
    .authorization_v2_for_request_with_body_reader(
        &Method::GET,
        &"http://upload.qiniup.com/".parse()?,
        &headers,
        &mut Cursor::new(b"{\"name\":\"test\"}"),
    );
```

#### 计算下载地址签名

```rust
use qiniu_credential::{Credential, prelude::*};
use std::time::Duration;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let url = "http://www.qiniu.com/?go=1".parse()?;
let url = credential
    .get(Default::default())?
    .sign_download_url(url, Duration::from_secs(3600));
println!("{}", url);
```

### 异步代码示例

#### 计算七牛鉴权签名 V1

```rust
use qiniu_credential::{Credential, HeaderValue, prelude::*};
use mime::APPLICATION_WWW_FORM_URLENCODED;
use std::io::Cursor;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let authorization = credential
    .async_get(Default::default()).await?
    .authorization_v1_for_request_with_async_body_reader(
        &"http://upload.qiniup.com/".parse()?,
        Some(&HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref())?),
        &mut Cursor::new(b"name=test&language=go"),
    ).await?;
```

#### 计算七牛鉴权签名 V2

```rust
use qiniu_credential::{Credential, Method, HeaderMap, HeaderValue, prelude::*};
use http::header::CONTENT_TYPE;
use mime::APPLICATION_JSON;
use std::io::Cursor;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let mut headers = HeaderMap::new();
headers.insert(CONTENT_TYPE, HeaderValue::from_str(APPLICATION_JSON.as_ref())?);
let authorization = credential
    .async_get(Default::default()).await?
    .authorization_v2_for_request_with_async_body_reader(
        &Method::GET,
        &"http://upload.qiniup.com/".parse()?,
        &headers,
        &mut Cursor::new(b"{\"name\":\"test\"}"),
    ).await?;
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
