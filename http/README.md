# Qiniu-Http

[![qiniu-http](https://img.shields.io/crates/v/qiniu-http.svg)](https://crates.io/crates/qiniu-http)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-http)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

为更高层的 HTTP 客户端提供基础 HTTP 请求接口 `HttpCaller`（同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能），
使不同的 HTTP 客户端基于相同的接口实现，
以便于七牛 API 调用层可以灵活切换 HTTP 客户端实现。
该接口库只关注 HTTP 调用相关逻辑，不包含七牛 API 调用相关逻辑。

## 安装

### 不启用异步接口

```toml
[dependencies]
qiniu-http = "0.1.3"
```

### 启用异步接口

```toml
[dependencies]
qiniu-http = { version = "0.1.3", features = ["async"] }
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
