# Qiniu-Apis

[![qiniu-apis](https://img.shields.io/crates/v/qiniu-apis.svg)](https://crates.io/crates/qiniu-apis)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-apis)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

这是一个基于 `qiniu-apis-specs` 自动生成的 Rust 库，基于 `qiniu-http-client`，用于调用七牛 HTTP API。
该库同时提供阻塞客户端和异步客户端，异步客户端则需要启用 `async` 功能。
该库致力于根据 [`qiniu-apis-specs`](https://github.com/qiniu/api-specs.git) 提供的 YAML 描述文件，在不理解业务逻辑的前提下，提供简单的封装方法方便用户正确调用 API。

该库可以通过启用不同的功能来选择不同的 HTTP 客户端实现，
例如可以通过启用 `ureq` 功能导入 `qiniu-ureq` 库作为 HTTP 客户端，
通过启用 `reqwest` 功能导入 `qiniu-reqwest` 库作为 HTTP 客户端，
通过启用 `isahc` 功能导入 `qiniu-isahc` 库作为 HTTP 客户端。
您也可以显式传入任何基于 `qiniu-http` 接口的 HTTP 客户端实现来提供给 `qiniu-apis` 使用。

## 安装

### 不启用异步接口，推荐使用 `ureq`

```toml
[dependencies]
qiniu-apis = { version = "0.1.3", features = ["ureq"] }
```

### 启用 Isahc 异步接口

```toml
[dependencies]
qiniu-apis = { version = "0.1.3", features = ["async", "isahc"] }
```

### 启用 Reqwest 异步接口

```toml
[dependencies]
qiniu-apis = { version = "0.1.3", features = ["async", "reqwest"] }
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

#### 创建存储空间

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{AllRegionsProvider, RegionsProvider, RegionsProviderEndpoints},
    storage::create_bucket::PathParams,
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let region = AllRegionsProvider::new(credential.to_owned())
    .get(Default::default())?;
Client::default()
    .storage()
    .create_bucket()
    .new_request(
        RegionsProviderEndpoints::new(&region),
        PathParams::default()
            .set_bucket_as_str("new-bucket-name")
            .set_region_as_str("z1"),
        credential,
    )
    .call()?;
```

#### 设置存储空间标签

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
    storage::set_bucket_taggings::{QueryParams, RequestBody, TagInfo, Tags},
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_name = "test-bucket";
let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
let mut tag1 = TagInfo::default();
tag1.set_key_as_str("tag_key1".to_owned());
tag1.set_value_as_str("tag_val1".to_owned());
let mut tag2 = TagInfo::default();
tag2.set_key_as_str("tag_key2".to_owned());
tag2.set_value_as_str("tag_val2".to_owned());
let mut tags = Tags::default();
tags.push_tag_info(tag1);
tags.push_tag_info(tag2);
let mut req_body = RequestBody::default();
req_body.set_tags(tags);
Client::default()
    .storage()
    .set_bucket_taggings()
    .new_request(RegionsProviderEndpoints::new(&region), credential)
    .query_pairs(QueryParams::default().set_bucket_as_str(bucket_name))
    .call(&req_body)?;
```

#### 列出存储空间标签

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
    storage::get_bucket_taggings::QueryParams,
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_name = "test-bucket";
let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
let tags = Client::default()
    .storage()
    .get_bucket_taggings()
    .new_request(RegionsProviderEndpoints::new(&region), credential)
    .query_pairs(QueryParams::default().set_bucket_name_as_str(bucket_name))
    .call()?
    .into_body()
    .get_tags()
    .to_tag_info_vec();
for tag in tags {
    println!("{}: {}", tag.get_key_as_str(), tag.get_value_as_str());
}
```

### 异步代码示例

#### 创建存储空间

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{AllRegionsProvider, RegionsProvider, RegionsProviderEndpoints},
    storage::create_bucket::PathParams,
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let region = AllRegionsProvider::new(credential.to_owned())
    .async_get(Default::default())
    .await?;
Client::default()
    .storage()
    .create_bucket()
    .new_async_request(
        RegionsProviderEndpoints::new(&region),
        PathParams::default()
            .set_bucket_as_str("new-bucket-name")
            .set_region_as_str("z1"),
        credential,
    )
    .call()
    .await?;
```

#### 设置存储空间标签

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
    storage::set_bucket_taggings::{QueryParams, RequestBody, TagInfo, Tags},
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_name = "test-bucket";
let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
let mut tag1 = TagInfo::default();
tag1.set_key_as_str("tag_key1".to_owned());
tag1.set_value_as_str("tag_val1".to_owned());
let mut tag2 = TagInfo::default();
tag2.set_key_as_str("tag_key2".to_owned());
tag2.set_value_as_str("tag_val2".to_owned());
let mut tags = Tags::default();
tags.push_tag_info(tag1);
tags.push_tag_info(tag2);
let mut req_body = RequestBody::default();
req_body.set_tags(tags);
Client::default()
    .storage()
    .set_bucket_taggings()
    .new_async_request(RegionsProviderEndpoints::new(&region), credential)
    .query_pairs(QueryParams::default().set_bucket_as_str(bucket_name))
    .call(&req_body)
    .await?;
```

#### 列出存储空间标签

```rust
use qiniu_apis::{
    credential::Credential,
    http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
    storage::get_bucket_taggings::QueryParams,
    Client,
};
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_name = "test-bucket";
let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
let tags = Client::default()
    .storage()
    .get_bucket_taggings()
    .new_async_request(RegionsProviderEndpoints::new(&region), credential)
    .query_pairs(QueryParams::default().set_bucket_name_as_str(bucket_name))
    .call()
    .await?
    .into_body()
    .get_tags()
    .to_tag_info_vec();
for tag in tags {
    println!("{}: {}", tag.get_key_as_str(), tag.get_value_as_str());
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
