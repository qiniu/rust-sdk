# Qiniu-SDK

[![qiniu-sdk](https://img.shields.io/crates/v/qiniu-sdk.svg)](https://crates.io/crates/qiniu-sdk)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-sdk)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

作为七牛所有 Rust SDK 插件的入口，可以通过启动功能来导入其他七牛 SDK。

## 功能描述

#### `utils`

允许通过 `qiniu_sdk::utils` 来访问 `qiniu-utils`。

#### `etag`

允许通过 `qiniu_sdk::etag` 来访问 `qiniu-etag`。

#### `credential`

允许通过 `qiniu_sdk::credential` 来访问 `qiniu-credential`。

#### `upload-token`

允许通过 `qiniu_sdk::upload_token` 来访问 `qiniu-upload-token`。

#### `http`

允许通过 `qiniu_sdk::http` 来访问 `qiniu-http`。

#### `http-client`

允许通过 `qiniu_sdk::http_client` 来访问 `qiniu-http-client`。

#### `apis`

允许通过 `qiniu_sdk::apis` 来访问 `qiniu-apis`。

#### `objects`

允许通过 `qiniu_sdk::objects` 来访问 `qiniu-objects-manager`。

#### `upload`

允许通过 `qiniu_sdk::upload` 来访问 `qiniu-upload-manager`。

#### `download`

允许通过 `qiniu_sdk::download` 来访问 `qiniu-download-manager`。

#### `async`

启用所有七牛 SDK 插件的异步接口。

#### `ureq`

导入 `qiniu-ureq` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::ureq` 来访问 `qiniu-ureq`。

#### `isahc`

导入 `qiniu-isahc` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::isahc` 来访问 `qiniu-isahc`。

#### `reqwest`

导入 `qiniu-reqwest` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::reqwest` 来访问 `qiniu-reqwest`。

#### `c_ares`

启用 `c-ares` 库作为 DNS 解析器。

#### `trust_dns`

启用 `trust-dns` 库作为 DNS 解析器。

#### `dns-over-https`

启用 `trust-dns` 库作为 DNS 解析器，并使用 DOH 协议。

#### `dns-over-tls`

启用 `trust-dns` 库作为 DNS 解析器，并使用 DOT 协议。

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
