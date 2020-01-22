# 设计思想

`qiniu-rust` 本身将 HTTP 客户端实现细节与 SDK 功能解耦合，自身仅实现七牛接口处理逻辑，并通过调用 `qiniu-rust-http` 定义的接口发送实际的 HTTP 请求。

在默认情况下，您在创建 `Config` 的实例时，必须为 `http_request_handler` 字段指定一个实现了 `qiniu-rust-http` 定义的接口的实例来处理 HTTP 请求，如果没有指定，则任何试图发送 HTTP 请求的接口都会以 Panic 的方式抛出错误。

如果对 `qiniu-rust` 开启了 `use-libcurl` 功能，则会为所有 `Config` 实例的 `http_request_handler` 字段设置为由 `qiniu-rust-with-libcurl` 定义的 HTTP 实现。`qiniu-rust-with-libcurl` 会使用 libcurl 库来处理 HTTP 逻辑。

注意：不要为 `qiniu-rust-http` 的接口提供任何基于 Rust [`http`](https://crates.io/crates/http) 库的实现，该 [`http`](https://crates.io/crates/http) 库无法处理七牛 API 定义的状态码。

## 模块设计

| 模块名                             | 模块描述                                                     |
| ---------------------------------- | ------------------------------------------------------------ |
| client | 七牛客户端，API 入口。 |
| config | 七牛客户端配置。 |
| credential | 七牛鉴权参数配置（也就是 AccessKey，SecretKey）。 |
| http | 实现 API 的 HTTP 请求的处理逻辑，包含 HTTP 请求/响应/回调接口库，自动域名管理，IP 地址解析和缓存，API 鉴权等功能。 |
| storage | 实现与云存储相关的功能。 |
| utils | SDK 基础实用库。 |
