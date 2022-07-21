# Qiniu-Etag

[![qiniu-etag](https://img.shields.io/crates/v/qiniu-etag.svg)](https://crates.io/crates/qiniu-etag)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/qiniu-etag)
[![Run Test Cases](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk.svg?label=release)](https://github.com/qiniu/rust-sdk/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk/blob/master/LICENSE)

## 概览

负责根据输入的数据计算七牛 Etag，适配 V1 和 V2 版本，同时提供阻塞接口和异步接口（异步接口需要启用 `async` 功能）

七牛 Etag 文档：https://developer.qiniu.com/kodo/1231/appendix

## 安装

### 不启用异步接口

```toml
[dependencies]
qiniu-etag = "0.1.3"
```

### 启用异步接口

```toml
[dependencies]
qiniu-etag = { version = "0.1.3", features = ["async"] }
```

## 代码示例

### 阻塞代码示例

#### Etag V1 计算示例

```rust
use qiniu_etag::{EtagV1, prelude::*};

let mut etag_v1 = EtagV1::new();
etag_v1.update(b"etag");
assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
```

#### Etag V1 计算输入流示例

```rust
use std::io::{copy, Cursor};
use qiniu_etag::{EtagV1, prelude::*};

let mut etag_v1 = EtagV1::new();
copy(&mut Cursor::new(b"etag"), &mut etag_v1)?;
assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
```

#### Etag V2 计算示例

```rust
use qiniu_etag::{EtagV2, prelude::*};

let mut etag_v2 = EtagV2::new();
etag_v2.update(b"hello");
etag_v2.update(b"world");
assert_eq!(etag_v2.finalize_fixed().as_slice(), b"ns56DcSIfBFUENXjdhsJTIvl3Rcu");
```

#### Etag V2 计算输入流示例

与 V1 不同的是，Etag V2 要求传入数据的分块方式

```rust
use qiniu_etag::etag_with_parts;
use std::io::Cursor;

assert_eq!(
    etag_with_parts(
        &mut Cursor::new(data_of_size(9 << 20)),
        &[1 << 22, 1 << 22, 1 << 20]
    )?,
    "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
);
```

### 异步代码示例

#### Etag V1 计算输入流示例

```rust
use futures::io::{copy, Cursor};
use qiniu_etag::{EtagV1, prelude::*};

let mut etag_v1 = EtagV1::new();
copy(&mut Cursor::new(b"etag"), &mut etag_v1).await?;
assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
```

#### Etag V2 计算输入流示例

```rust
use qiniu_etag::async_etag_of;
use futures::io::Cursor;

assert_eq!(
    async_etag_of(
        &mut Cursor::new(data_of_size(9 << 20)),
        &[1 << 22, 1 << 22, 1 << 20]
    ).await?,
    "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
);
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
