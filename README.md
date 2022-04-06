# Qiniu Resource Storage SDK for Rust

[![Run Test Cases](https://github.com/bachue/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/bachue/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/bachue/rust-sdk.svg?label=release)](https://github.com/bachue/rust-sdk/releases)

## 概要

Qiniu SDK for Rust 包含以下特性：

- 通过提供多个不同的 Crate，为不同层次的开发都提供了方便易用的编程接口。
- 同时提供阻塞 IO 接口和基于 Async/Await 的异步 IO 接口。
- 提供大量的可供二次开发的 Trait，方便灵活定制，例如 HTTP 客户端提供了 `ureq`，`reqwest` 和 `isahc` 三种不同的库实现，也可以基于 `qiniu-http` 自行定制开发接入其他 HTTP 客户端实现；又例如 DNS 客户端提供了 `libc`，`c-ares`，`trust-dns` 三种不同的库实现，也可以基于 [Resolver](https://docs.rs/qiniu-http-client/latest/qiniu_http_client/trait.Resolver.html) 自行定制开发接入其他 DNS 客户端实现。

## 安装

Qiniu SDK for Rust 包含以下 Crates:

| Crate 名称      | 描述 |
| ----------- | ----------- |
| [![qiniu-etag](https://img.shields.io/crates/v/qiniu-etag.svg)](https://crates.io/crates/qiniu-etag)      | Etag 算法库，实现七牛 Etag 算法       |
| [![qiniu-credential](https://img.shields.io/crates/v/qiniu-credential.svg)](https://crates.io/crates/qiniu-credential)      | 七牛认证库，实现七牛认证接口以及签名相关算法       |
| [![qiniu-upload-token](https://img.shields.io/crates/v/qiniu-upload-token.svg)](https://crates.io/crates/qiniu-upload-token)      | 七牛上传凭证，实现七牛上传策略和上传凭证接口以及相关算法      |
| [![qiniu-http](https://img.shields.io/crates/v/qiniu-http.svg)](https://crates.io/crates/qiniu-http)      | 七牛客户端 HTTP 接口，为不同的 HTTP 客户端实现提供相同的基础接口      |
| [![qiniu-ureq](https://img.shields.io/crates/v/qiniu-ureq.svg)](https://crates.io/crates/qiniu-ureq)      | 基于 [Ureq](https://docs.rs/ureq) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-isahc](https://img.shields.io/crates/v/qiniu-isahc.svg)](https://crates.io/crates/qiniu-isahc)      | 基于 [Isahc](https://docs.rs/isahc) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-reqwest](https://img.shields.io/crates/v/qiniu-reqwest.svg)](https://crates.io/crates/qiniu-reqwest)      | 基于 [Reqwest](https://docs.rs/reqwest) 库实现七牛客户端 HTTP 接口      |
| [![qiniu-http-client](https://img.shields.io/crates/v/qiniu-http-client.svg)](https://crates.io/crates/qiniu-http-client)      | 基于 [qiniu-http](https://docs.rs/qiniu-http) 提供具有重试功能的 HTTP 客户端       |
| [![qiniu-apis](https://img.shields.io/crates/v/qiniu-apis.svg)](https://crates.io/crates/qiniu-apis)      | 实现七牛 API 调用客户端接口       |
| [![qiniu-objects-manager](https://img.shields.io/crates/v/qiniu-objects-manager.svg)](https://crates.io/crates/qiniu-objects-manager)      | 实现七牛对象相关管理接口，包含对象的列举和操作       |
| [![qiniu-upload-manager](https://img.shields.io/crates/v/qiniu-upload-manager.svg)](https://crates.io/crates/qiniu-upload-manager)      | 实现七牛对象上传功能       |

## 最低支持的 Rust 版本（MSRV）

1.56.0

## 联系我们

- 如果需要帮助，请提交工单（在portal右侧点击咨询和建议提交工单，或者直接向 support@qiniu.com 发送邮件）
- 如果有什么问题，可以到问答社区提问，[问答社区](http://qiniu.segmentfault.com/)
- 更详细的文档，见[官方文档站](http://developer.qiniu.com/)
- 如果发现了bug， 欢迎提交 [issue](https://github.com/bachue/rust-sdk/issues)
- 如果有功能需求，欢迎提交 [issue](https://github.com/bachue/rust-sdk/issues)
- 如果要提交代码，欢迎提交 Pull Request
- 欢迎关注我们的[微信](https://www.qiniu.com/contact) [微博](http://weibo.com/qiniutek)，及时获取动态信息。

## 代码许可

This project is licensed under the [MIT license].

[MIT license]: https://github.com/bachue/rust-sdk/blob/master/LICENSE
