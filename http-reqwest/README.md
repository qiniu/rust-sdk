# Qiniu-Reqwest

[![qiniu-reqwest](https://img.shields.io/crates/v/qiniu-reqwest.svg)](https://crates.io/crates/qiniu-reqwest)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-reqwest)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

基于 Reqwest 库提供 HTTP 客户端接口实现（分别实现阻塞接口和异步接口，异步实现则需要启用 `tokio-runtime` 功能）

需要注意的是，如果使用阻塞接口，则必须使用 `SyncClient`，而如果使用异步接口则必须使用 `AsyncClient`，二者不能混用。

## 安装

### 不启用异步接口

```toml
[dependencies]
qiniu-reqwest = "0.3.0"
```

### 启用异步接口

```toml
[dependencies]
qiniu-reqwest = { version = "0.3.0", features = ["tokio-runtime"] }
```

## 最低支持的 Rust 版本（MSRV）

1.70.0

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
