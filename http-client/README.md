# Qiniu-Http-Client

[![qiniu-http-client](https://img.shields.io/crates/v/qiniu-http-client.svg)](https://crates.io/crates/qiniu-http-client)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http-client)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

基于 `qiniu-http` 提供具有重试功能的 HTTP 客户端
（同时提供阻塞客户端和异步客户端，异步客户端则需要启用 `async` 功能），
通过对七牛特有的状态码和响应头进行合理的处理和重试，竭力保证七牛 API 可以调用成功。

该接口库可以通过启用不同的功能来选择不同的客户端实现，
例如可以通过启用 `ureq` 功能导入 `qiniu-ureq` 库作为 HTTP 客户端，
通过启用 `reqwest` 功能导入 `qiniu-reqwest` 库作为 HTTP 客户端，
通过启用 `isahc` 功能导入 `qiniu-isahc` 库作为 HTTP 客户端，
您也可以显式传入任何基于 `qiniu-http` 接口的 HTTP 客户端实现来提供给 `qiniu-http-client` 使用。

`qiniu-http-client` 提供的功能主要分为两个大类：区域相关和 HTTP 客户端逻辑相关。

### 区域相关

`qiniu-http-client` 表示一个服务器地址有两种方式，
一种是 IP 地址加端口号，用 `IpAddrWithPort` 表示（与 Rust 自带的 `std::net::SocketAddr` 不同的是，端口号是可选参数），
另一种是域名加端口号，用 `DomainWithPort` 表示。这两个类型都可以用 `Endpoint` 这个枚举类来统一表示。

对七牛来说，大部分服务都可以有多个域名或是服务器 IP 地址，因此用 `Endpoints` 来表示一个服务的多个域名和服务器 IP 地址。

无论是七牛的公有云还是私有云，一个区域包含多个服务，用 `Region` 来表示一个区域，用于存储该区域的 ID 和所有服务的 `Endpoints`。
区域并不总是需要用户静态配置，可以通过实现 `RegionsProvider` 接口来定义区域获取的不同方式，
`StaticRegionsProvider` 实现了最基础的静态配置区域；
`AllRegionsProvider` 实现了查询七牛所有区域的功能，支持内存缓存和文件系统缓存；
`BucketRegionsQueryer` 可以用来查询七牛某个存储空间的所在区域和相关的多活区域，支持内存缓存和文件系统缓存；

在调用一个七牛服务的 API 时，需要传入所有服务器可用的域名或服务器 IP 地址以便于客户端重试，如果只能静态配置将十分麻烦，
因此我们也提供 `EndpointsProvider` 供用户实现不同的 `Endpoints` 获取方式，
`RegionsProviderEndpoints` 可以从任意一个 `RegionsProvider` 实现中获取 `Endpoints`；
`BucketDomainsQueryer` 可以用来查询七牛某个存储空间绑定的域名，支持内存缓存和文件系统缓存。

### HTTP 客户端逻辑相关

#### `qiniu_http::HttpCaller`

`qiniu_http::HttpCaller` 提供 HTTP 请求接口（同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能）。
`qiniu_ureq::Client` 提供基于 `ureq` 库的 HTTP 客户端（需要启用 `ureq` 功能），特点是代码精简，依赖简单，但不支持异步接口；
`qiniu_reqwest::SyncClient` 和 `qiniu_reqwest::AsyncClient` 提供基于 `reqwest` 库的 HTTP 客户端
（需要启用 `reqwest` 功能，如果需要用到异步接口还需要额外启用 `async` 功能），
特点是支持阻塞接口和异步接口，但两个客户端不能混用，
即 `qiniu_reqwest::SyncClient` 只能用于发送阻塞请求，而 `qiniu_reqwest::AsyncClient` 只能用来发送异步请求，
且由于 `reqwest` 库自身基于异步接口实现，因此即使不启用 `async` 功能，也会用线程启动 `tokio` 异步环境驱动 HTTP 请求发送；
`qiniu_isahc::Client` 提供基于 `isahc` 库的 HTTP 客户端
（需要启用 `reqwest` 功能，如果需要用到异步接口还需要额外启用 `async` 功能），
特点是功能全面，且同时支持阻塞接口和异步接口，但依赖原生的 [`libcurl`](https://curl.se/libcurl/) 库，
且由于 `isahc` 库自身基于异步接口实现，因此即使不启用 `async` 功能，也会用线程启动 `tokio` 异步环境驱动 HTTP 请求发送；
可以通过配置 `HttpClientBuilder::http_caller` 来指定使用哪个客户端。如果不指定，默认通过当前启用的功能来判定。

#### `Resolver`

`Resolver` 提供域名解析的接口（同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能），可以将一个域名解析为 IP 地址列表。
`qiniu-http-client` 提供多种域名解析的实现，
`SimpleResolver` 提供最简单的，基于 [`libc`](https://man7.org/linux/man-pages/man7/libc.7.html) 库的域名解析实现；
`CAresResolver` （需要启用 `c_ares` 功能）提供基于 [`c-ares`](https://c-ares.org/) 库的域名解析实现；
`TrustDnsResolver` （需要同时启用 `trust_dns` 功能和 `async` 功能）提供基于 [`trust-dns`](https://trust-dns.org/) 库的域名解析实现；
`TimeoutResolver` 为任意一个 `Resolver` 实现提供超时功能，不过该实现不是很高效，如果 `Resolver` 实现本身就带有超时功能，还是尽量使用自带的超时功能更好；
`ShuffledResolver` 可以将任意一个 `Resolver` 实现提供打乱解析结果的功能，便于在客户端层面实现简单的服务器的负载均衡功能；
`ChainedResolver` 提供对多个 `Resolver` 的链式调用，可以依次尝试不同的 `Resolver` 实现直到获得一个有效的解析结果为止；
`CachedResolver` 提供对 `Resolver` 的缓存功能，支持内存缓存和文件系统缓存；
可以通过配置 `HttpClientBuilder::resolver` 来指定使用哪个解析器，如果不指定，默认通过当前启用的功能来判定，并使用 `CachedResolver` 和 `ShuffledResolver` 对其进行包装。

#### `Chooser`

`Chooser` 提供 IP 地址选择的功能，以及提供反馈接口以修正自身选择逻辑的功能（同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能）。
`qiniu-http-client` 提供多种选择器的实现，
`DirectChooser` 提供最直接的选择器，即不做任何筛选，直接将所有传入的 IP 地址返回；
`IpChooser` 提供包含 IP 地址黑名单的选择器，即反馈 API 调用失败，则将所有相关 IP 地址冻结一段时间，在这段时间内，这些 IP 地址将会被过滤，不会被选择到；
`SubnetChooser` 提供包含子网黑名单的选择器，即反馈 API 调用失败，则将所有相关 IP 地址所在的子网冻结一段时间，在这段时间内，任何与这些 IP 地址处于同一子网内的所有 IP 地址都将会被过滤，不会被选择到；
`ShuffledChooser` 可以将任意一个 `Chooser` 实例提供打乱选择结果的功能，便于在客户端层面实现简单的服务器的负载均衡功能；
`NeverEmptyHandedChooser` 确保 `Chooser` 实例不会因为所有可选择的 IP 地址都被屏蔽而导致 HTTP 客户端直接返回错误，在内置的 `Chooser` 没有返回结果时，将会随机返回一定比例的 IP 地址供 HTTP 客户端做一轮尝试。
可以通过配置 `HttpClientBuilder::chooser` 来指定使用哪个选择器，如果不指定，默认使用 `SubnetChooser`，并使用 `ShuffledChooser` 和 `NeverEmptyHandedChooser` 对其进行包装。

#### `RequestRetrier`

`RequestRetrier` 根据 HTTP 客户端返回的错误，决定是否重试请求，重试决定由 `RetryDecision` 定义。
`qiniu-http-client` 提供多种重试器的实现，
`NeverRetrier` 总是返回不重试的决定；
`ErrorRetrier` 致力于通过七牛 API 返回的状态码作出正确的重试决定；
`LimitedRetrier` 为一个 `RequestRetrier` 实例增加重试次数上限，即重试次数到达上限时，无论错误是什么，都切换服务器地址或不再予以重试。
可以通过配置 `HttpClientBuilder::request_retrier` 来指定使用哪个重试器，如果不指定，默认使用 `ErrorRetrier`，并使用 `LimitedRetrier` 对其进行包装。

#### `Backoff`

`Backoff` 根据 HTTP 客户端返回的错误和 `RequestRetrier` 返回的重试决定，决定退避时长。
`qiniu-http-client` 提供多种退避器的实现，
`FixedBackoff` 总是返回固定的退避时长；
`ExponentialBackoff` 根据重试次数返回指数级增长的退避时长；
`LimitedBackoff` 为一个 `Backoff` 实例增加上限和下限，即如果基础的 `Backoff` 实例返回的退避时长超过限制，则返回极限值；
`RandomizedBackoff` 为一个 `Backoff` 实例返回的退避时长增加随机范围，即返回的退避时长随机化。
可以通过配置 `HttpClientBuilder::backoff` 来指定使用哪个退避器，如果不指定，默认使用 `ExponentialBackoff`，并使用 `LimitedBackoff` 和 `RandomizedBackoff` 对其进行包装。

## 安装

### 不启用异步接口，推荐使用 `ureq`

```toml
[dependencies]
qiniu-http-client = { version = "0.1.3", features = ["ureq"] }
```

### 启用 Isahc 异步接口

```toml
[dependencies]
qiniu-http-client = { version = "0.1.3", features = ["async", "isahc"] }
```

### 启用 Reqwest 异步接口

```toml
[dependencies]
qiniu-http-client = { version = "0.1.3", features = ["async", "reqwest"] }
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

#### 私有云获取当前账户的 Buckets 列表

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, HttpClient, Region, RegionsProviderEndpoints, ServiceName};

let region = Region::builder("z0")
    .add_uc_preferred_endpoint("uc-qos.pocdemo.qiniu.io".parse()?)
    .build();
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_names: Vec<String> = HttpClient::default()
    .get(&[ServiceName::Uc], RegionsProviderEndpoints::new(region))
    .use_https(false)
    .authorization(Authorization::v2(credential))
    .accept_json()
    .path("/buckets")
    .call()
    .parse_json()
    .into_body();
```

#### 公有云获取对象信息

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, BucketRegionsQueryer, HttpClient, RegionsProviderEndpoints, ServiceName};
use serde_json::Value;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let value: Value = HttpClient::default()
    .get(
        &[ServiceName::Rs],
        RegionsProviderEndpoints::new(
            BucketRegionsQueryer::new().query(credential.access_key().to_owned(), "test-bucket"),
        ),
    )
    .path("/stat/dGVzdC1idWNrZXQ6dGVzdC1rZXk=")
    .authorization(Authorization::v2(credential))
    .accept_json()
    .call()
    .parse_json()
    .into_body();
```

#### 公有云私有空间下载文件（存储空间必须绑定至少一个域名）

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, BucketDomainsQueryer, HttpClient};

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let response = HttpClient::default()
    .get(
        &[],
        BucketDomainsQueryer::new().query(credential.to_owned(), "test-bucket"),
    )
    .path("/test-key")
    .use_https(false)
    .authorization(Authorization::download(credential))
    .call();
```


### 异步代码示例

#### 私有云获取当前账户的 Buckets 列表

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, HttpClient, Region, RegionsProviderEndpoints, ServiceName};

let region = Region::builder("z0")
    .add_uc_preferred_endpoint("uc-qos.pocdemo.qiniu.io".parse()?)
    .build();
let credential = Credential::new("abcdefghklmnopq", "1234567890");
let bucket_names: Vec<String> = HttpClient::default()
    .async_get(&[ServiceName::Uc], RegionsProviderEndpoints::new(region))
    .use_https(false)
    .authorization(Authorization::v2(credential))
    .accept_json()
    .path("/buckets")
    .call()
    .await?
    .parse_json()
    .await?
    .into_body();
```

#### 公有云获取对象信息

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, BucketRegionsQueryer, HttpClient, RegionsProviderEndpoints, ServiceName};
use serde_json::Value;

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let value: Value = HttpClient::default()
    .async_get(
        &[ServiceName::Rs],
        RegionsProviderEndpoints::new(
            BucketRegionsQueryer::new().query(credential.access_key().to_owned(), "test-bucket"),
        ),
    )
    .path("/stat/dGVzdC1idWNrZXQ6dGVzdC1rZXk=")
    .authorization(Authorization::v2(credential))
    .accept_json()
    .call()
    .await?
    .parse_json()
    .await?
    .into_body();
```

#### 公有云私有空间下载文件（存储空间必须绑定至少一个域名）

```rust
use qiniu_credential::Credential;
use qiniu_http_client::{Authorization, BucketDomainsQueryer, HttpClient};

let credential = Credential::new("abcdefghklmnopq", "1234567890");
let response = HttpClient::default()
    .async_get(
        &[],
        BucketDomainsQueryer::new().query(credential.to_owned(), "test-bucket"),
    )
    .path("/test-key")
    .use_https(false)
    .authorization(Authorization::download(credential))
    .call()
    .await?;
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
