# Change log

## v0.1.4

- `ObjectsManagerBuilder` / `UploadManagerBuilder` 的 `use_https` 方法也对内置的 `BucketRegionsQueryerBuilder` 生效
- `ListIter` / `ListStream` 添加 `marker` 方法，获取最近一次列举返回的位置标记

## v0.1.3

- 修复编译 `ureq` 出现 `pattern InsecureRequestHttpsOnly not covered` 的错误
- 升级 `fs4` 否则可能导致在 Linux 上编译失败
- 为 `SubnetChooserBuilder` 增加安全的设置 IP 地址子网掩码前缀长度的方法
- 导出多个遗漏的公开类型
- 解决 `qiniu_http::Request::from_parts_and_body` 无法使用的 bug

## v0.1.2
- 对象存储，管理类 API 发送请求时增加 [X-Qiniu-Date](https://developer.qiniu.com/kodo/3924/common-request-headers) （生成请求的时间） header
- 简化了 `http_client::RegionBuilder` / `http_client::EndpointsBuilder` 设置多个终端地址的方法
- 提供 `qiniu_http::set_library_user_agent()` 给第三方库设置用户代理，方便所有 User Agent 内包含第三方库信息
- 修复了部分已知问题

## v0.1.1

- 增加 `qiniu-download-manager` 插件负责对象的下载
- 修复了 `Credential::sign_download_url()` 的 `lifetime` 参数实现不正确的 bug
- 简化了设置 HTTP 协议的代码
- 优化 HTTP Header 相关接口

## v0.1.0

- 首个正式版本发布
