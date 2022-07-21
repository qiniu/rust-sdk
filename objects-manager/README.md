# Qiniu-Objects-Manager

[![qiniu-objects-manager](https://img.shields.io/crates/v/qiniu-objects-manager.svg)](https://crates.io/crates/qiniu-objects-manager)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-objects-manager)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

基于 `qiniu-apis` 提供针对七牛对象的管理功能 （同时提供阻塞客户端和异步客户端，异步客户端则需要启用 `async` 功能），
主要负责七牛对象的列举和操作。

## 安装

### 不启用异步接口，推荐使用 `ureq`

```toml
[dependencies]
qiniu-objects-manager = { version = "0.1.3", features = ["ureq"] }
```

### 启用 Isahc 异步接口

```toml
[dependencies]
qiniu-objects-manager = { version = "0.1.3", features = ["async", "isahc"] }
```

### 启用 Reqwest 异步接口

```toml
[dependencies]
qiniu-objects-manager = { version = "0.1.3", features = ["async", "reqwest"] }
```

### 其他功能

#### `c_ares`

启用 `c-ares` 库作为 DNS 解析器

#### `trust_dns`

启用 `trust-dns` 库作为 DNS 解析器

#### `dns-over-https`

启用 `trust-dns` 库作为 DNS 解析器，并使用 DOH 协议

#### `dns-over-tls`

启用 `trust-dns` 库作为 DNS 解析器，并使用 DOT 协议

## 代码示例

### 阻塞代码示例

#### 对象元信息获取

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");

let response = bucket.stat_object("test-key").call()?;
let object = response.into_body();
println!("fsize: {}", object.get_size_as_u64());
println!("hash: {}", object.get_hash_as_str());
println!("mime_type: {}", object.get_mime_type_as_str());
```

#### 对象批量元信息获取

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager, OperationProvider};
use futures::stream::TryStreamExt;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.stat_object("test-file-1"));
ops.add_operation(bucket.stat_object("test-file-2"));
ops.add_operation(bucket.stat_object("test-file-3"));
ops.add_operation(bucket.stat_object("test-file-4"));
ops.add_operation(bucket.stat_object("test-file-5"));
let mut iter = ops.call();
while let Some(object) = iter.next() {
    let object = object?;
    println!("fsize: {:?}", object.get_size_as_u64());
    println!("hash: {:?}", object.get_hash_as_str());
    println!("mime_type: {:?}", object.get_mime_type_as_str());
}
```

#### 对象列举

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
use futures::stream::TryStreamExt;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");
let mut iter = bucket.list().iter();
while let Some(object) = iter.next() {
    let object = object?;
    println!("fsize: {:?}", object.get_size_as_u64());
    println!("hash: {:?}", object.get_hash_as_str());
    println!("mime_type: {:?}", object.get_mime_type_as_str());
}
```

### 异步代码示例

#### 对象元信息获取

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");

let response = bucket.stat_object("test-key").async_call().await?;
let object = response.into_body();
println!("fsize: {}", object.get_size_as_u64());
println!("hash: {}", object.get_hash_as_str());
println!("mime_type: {}", object.get_mime_type_as_str());
```

#### 对象批量元信息获取

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager, OperationProvider};
use futures::stream::TryStreamExt;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");
let mut ops = bucket.batch_ops();
ops.add_operation(bucket.stat_object("test-file-1"));
ops.add_operation(bucket.stat_object("test-file-2"));
ops.add_operation(bucket.stat_object("test-file-3"));
ops.add_operation(bucket.stat_object("test-file-4"));
ops.add_operation(bucket.stat_object("test-file-5"));
let mut stream = ops.async_call();
while let Some(object) = stream.try_next().await? {
    println!("fsize: {:?}", object.get_size_as_u64());
    println!("hash: {:?}", object.get_hash_as_str());
    println!("mime_type: {:?}", object.get_mime_type_as_str());
}
```

#### 对象列举

```rust
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
use futures::stream::TryStreamExt;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let object_manager = ObjectsManager::new(credential);
let bucket = object_manager.bucket("test-bucket");
let mut stream = bucket.list().stream();
while let Some(object) = stream.try_next().await? {
    println!("fsize: {:?}", object.get_size_as_u64());
    println!("hash: {:?}", object.get_hash_as_str());
    println!("mime_type: {:?}", object.get_mime_type_as_str());
}
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
