# Change log

## v0.1.2
- 对象存储，管理类 API 发送请求时增加 [X-Qiniu-Date](https://developer.qiniu.com/kodo/3924/common-request-headers) （生成请求的时间） header
- 简化了 http_client::RegionBuilder / http_client::EndpointsBuilder 设置多个终端地址的方法
- 提供 qiniu_http::set_library_user_agent() 给第三方库设置用户代理，方便所有 User Agent 内包含第三方库信息
- 修复了部分已知问题

## v0.1.1

- 增加 `qiniu-download-manager` 插件负责对象的下载
- 修复了 `Credential::sign_download_url()` 的 `lifetime` 参数实现不正确的 bug
- 简化了设置 HTTP 协议的代码
- 优化 HTTP Header 相关接口

## v0.1.0

- 首个正式版本发布
