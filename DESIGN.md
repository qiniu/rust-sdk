# 设计思想

此 SDK 并非仅为 Rust 语言设计，而是利用 Rust 语言构建 SDK 核心库，然后对外提供多种编程语言接口的多平台多接口的通用 SDK。

由于核心均基于同一个库，外层仅仅只需要构建 API 绑定层，因此大幅改善了 SDK 的开发效率。

## 模块设计

### 功能模块

| 模块名                                                       | 模块描述                                                     |
| ------------------------------------------------------------ | ------------------------------------------------------------ |
| [qiniu-rust](qiniu-rust/README.md)                           | SDK 功能核心模块，采用 Rust 语言实现，提供 Rust SDK 所有功能 |
| [qiniu-rust-http](qiniu-rust-http/README.md)                 | 定义了 HTTP 客户端接口，采用 Rust 语言实现，由 qiniu-rust 调用。该模块用于解耦 SDK 功能和 HTTP 客户端实现。 |
| [qiniu-rust-with-libcurl](qiniu-rust-with-libcurl/README.md) | 基于 `libcurl` 的 HTTP 客户端实现，采用 Rust 语言实现，实现 `qiniu-rust-http` 定义的接口，由启用了 `use-libcurl` 功能的 `qiniu-rust` 调用。 |
| [qiniu-c](qiniu-c/README.md)                                 | 为 `qiniu-rust` 提供 C 接口，采用 Rust 语言实现（但测试用例采用 C 语言实现）。 |
| [qiniu-ruby](qiniu-ruby/README.md)                           | 为 `qiniu-c` 提供 Ruby 接口，采用 Ruby 语言实现。            |

### 工具模块

| 模块名                             | 模块描述                                                     |
| ---------------------------------- | ------------------------------------------------------------ |
| [qiniu-c-translator](qiniu-c-translator/README.md) | 将 `qiniu-c` 提供的 C 接口翻译为多种编程语言的 C 绑定接口 |

### 测试模块

| 模块名                                                   | 模块描述                                                     |
| -------------------------------------------------------- | ------------------------------------------------------------ |
| [qiniu-rust-test](qiniu-rust-test/README.md)             | `qiniu-rust` 集成测试模块，将基于七牛公有云存储测试 `qiniu-rust` 的功能实现 |
| [qiniu-rust-test-utils](qiniu-rust-test-utils/README.md) | 为 `qiniu-rust` 和 `qiniu-rust-test-utils` 提供公共测试库函数 |

1. 以 Ruby 接口为例。当用户调用 Ruby 接口时，`qiniu-ruby` 将调用由 `qiniu-c-translator` 自动翻译的的 C 绑定接口，从而调用到 `qiniu-c` 的接口。
2. `qiniu-c` 接口将调用 `qiniu-rust` 的功能实现。
3. 对于需要发送 HTTP 请求的接口，`qiniu-rust` 调用 `qiniu-rust-http` 定义的 HTTP 客户端接口，从而调用到 `qiniu-rust-with-libcurl` 或其他客户端实现来处理 SDK 的 HTTP 请求。

## 实现限制

- 在 WebAssembly 普及前，暂不考虑提供面向浏览器的前端 SDK。
- 由于 Rust 编译后库尺寸较大，可能难以被移动端 SDK 用户接受，因此暂时不提供 Android SDK 和 iOS SDK 接口。
